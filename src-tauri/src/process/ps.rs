//! 解析 `ps` 輸出，找出各 tty 的前景程式並判斷類型。
//!
//! 前景程式＝前景程序群組（`pgid == tpgid`）的領頭（`pid == pgid`）；
//! 領頭已結束時取群組中 pid 最小者。這裡只負責查詢，
//! 工作階段只來自 `TerminalBackend::discover()`。

use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Command;

use crate::error::CueError;
use crate::model::{normalize_tty, SessionKind};

/// 一輪探索只執行一次；`-ww` 避免 `args` 被欄寬截斷。
const PS_PATH: &str = "/bin/ps";
const PS_ARGS: [&str; 4] = ["-A", "-ww", "-o", "pid=,ppid=,pgid=,tpgid=,tty=,args="];

const SHELLS: [&str; 4] = ["zsh", "bash", "fish", "sh"];
const CLAUDE_PACKAGE: &str = "@anthropic-ai/claude-code";
const GEMINI_PACKAGE: &str = "@google/gemini-cli";

/// `ps` 的單筆程序資訊。只保留有 tty 的程序。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcInfo {
    pub pid: i32,
    pub ppid: i32,
    pub pgid: i32,
    pub tpgid: i32,
    /// 經 [`normalize_tty`] 正規化的 tty。
    pub tty: String,
    pub args: String,
}

/// 某個 tty 的前景程式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Foreground {
    /// 前景程序群組領頭的 pid；cwd 也只取這支程序。
    pub pid: i32,
    pub kind: SessionKind,
}

/// 執行一次 `ps` 並解析，排除 Cue 自己的子程序。
pub fn snapshot() -> Result<Vec<ProcInfo>, CueError> {
    let output = Command::new(PS_PATH)
        .args(PS_ARGS)
        .output()
        .map_err(|e| CueError::Io(format!("無法執行 ps：{e}")))?;
    if !output.status.success() {
        return Err(CueError::Io(format!("ps 結束狀態異常：{}", output.status)));
    }
    let self_pid = i32::try_from(std::process::id()).unwrap_or(i32::MAX);
    Ok(parse_ps(&String::from_utf8_lossy(&output.stdout), self_pid))
}

/// 解析 `ps -o pid=,ppid=,pgid=,tpgid=,tty=,args=` 的輸出。
///
/// 前五欄以空白切割，剩餘整段為 `args`（可能含空白）。沒有 tty 的程序（`??`）、
/// 格式不符的行，以及 `self_pid` 的子程序（Cue 自己執行的 `ps`、`osascript`）一律略過。
pub fn parse_ps(output: &str, self_pid: i32) -> Vec<ProcInfo> {
    output
        .lines()
        .filter_map(parse_line)
        .filter(|p| p.ppid != self_pid)
        .collect()
}

fn parse_line(line: &str) -> Option<ProcInfo> {
    let mut rest = line;
    let mut fields = [""; 5];
    for field in &mut fields {
        rest = rest.trim_start();
        let end = rest.find(char::is_whitespace)?;
        *field = &rest[..end];
        rest = &rest[end..];
    }
    let args = rest.trim();
    if args.is_empty() {
        return None;
    }
    Some(ProcInfo {
        pid: fields[0].parse().ok()?,
        ppid: fields[1].parse().ok()?,
        pgid: fields[2].parse().ok()?,
        tpgid: fields[3].parse().ok()?,
        tty: normalize_tty(fields[4])?,
        args: args.to_string(),
    })
}

/// 以 tty 查詢前景程式。結果只供查詢 `discover()` 回報的 tty，
/// 不得用來為其他 tty 產生工作階段。
pub fn foreground_by_tty(procs: &[ProcInfo]) -> HashMap<String, Foreground> {
    // tty → pgid → 前景群組成員
    let mut groups: HashMap<&str, HashMap<i32, Vec<&ProcInfo>>> = HashMap::new();
    for p in procs.iter().filter(|p| p.pgid == p.tpgid && p.tpgid > 0) {
        groups.entry(p.tty.as_str()).or_default().entry(p.pgid).or_default().push(p);
    }

    groups
        .into_iter()
        .filter_map(|(tty, by_pgid)| {
            // 正常情況每個 tty 只有一個前景群組；`ps` 取樣競態時優先取有領頭者，再取 pgid 最小者。
            let (_, members) = by_pgid
                .into_iter()
                .min_by_key(|(pgid, members)| (!members.iter().any(|p| p.pid == *pgid), *pgid))?;
            let leader = members
                .iter()
                .find(|p| p.pid == p.pgid)
                .or_else(|| members.iter().min_by_key(|p| p.pid))?;
            let kind = classify(leader, &members);
            Some((tty.to_string(), Foreground { pid: leader.pid, kind }))
        })
        .collect()
}

/// 判斷前景程式類型。先看領頭；領頭是包裝程式（`npm exec`、`npx`、腳本不是
/// claude／gemini 的 `node`）時，再往同一群組的子孫程序找 claude／gemini。
pub fn classify(leader: &ProcInfo, group: &[&ProcInfo]) -> SessionKind {
    match detect(&leader.args) {
        Detected::Kind(kind) => kind,
        Detected::Wrapper(name) => search_group(leader, group).unwrap_or(SessionKind::Other(name)),
    }
}

/// 以領頭為起點廣度優先走訪子孫，再補上群組中其餘程序（領頭已結束時會被改掛到 launchd）。
fn search_group(leader: &ProcInfo, group: &[&ProcInfo]) -> Option<SessionKind> {
    let mut seen: HashSet<i32> = HashSet::from([leader.pid]);
    let mut order: Vec<&ProcInfo> = Vec::with_capacity(group.len());
    let mut queue = VecDeque::from([leader.pid]);
    while let Some(parent) = queue.pop_front() {
        let mut children: Vec<&ProcInfo> =
            group.iter().copied().filter(|p| p.ppid == parent && !seen.contains(&p.pid)).collect();
        children.sort_by_key(|p| p.pid);
        for child in children {
            seen.insert(child.pid);
            queue.push_back(child.pid);
            order.push(child);
        }
    }
    let mut rest: Vec<&ProcInfo> = group.iter().copied().filter(|p| !seen.contains(&p.pid)).collect();
    rest.sort_by_key(|p| p.pid);
    order.extend(rest);

    order.into_iter().find_map(|p| match detect(&p.args) {
        Detected::Kind(kind @ (SessionKind::Claude | SessionKind::Gemini)) => Some(kind),
        _ => None,
    })
}

enum Detected {
    Kind(SessionKind),
    /// 包裝程式，需往子孫程序找；內容為執行檔名稱，找不到時當作 `Other`。
    Wrapper(String),
}

fn detect(args: &str) -> Detected {
    let mut tokens = args.split_whitespace();
    let Some(exe) = tokens.next() else {
        return Detected::Kind(SessionKind::Other(String::new()));
    };
    let name = basename(exe.strip_prefix('-').unwrap_or(exe));

    match name {
        "claude" => Detected::Kind(SessionKind::Claude),
        _ if SHELLS.contains(&name) => Detected::Kind(SessionKind::Zsh),
        "node" | "bun" => match first_positional(tokens) {
            Some(script) if script.contains(CLAUDE_PACKAGE) => Detected::Kind(SessionKind::Claude),
            Some(script) if basename(script) == "gemini" || script.contains(GEMINI_PACKAGE) => {
                Detected::Kind(SessionKind::Gemini)
            }
            _ => Detected::Wrapper(name.to_string()),
        },
        "npm" => match first_positional(tokens.by_ref()) {
            Some("exec") => package_kind(first_positional(tokens), name),
            _ => Detected::Kind(SessionKind::Other(name.to_string())),
        },
        "npx" => package_kind(first_positional(tokens), name),
        _ => Detected::Kind(SessionKind::Other(name.to_string())),
    }
}

/// `npm exec`／`npx` 執行的套件（可帶 `@版本`）。
fn package_kind(package: Option<&str>, wrapper: &str) -> Detected {
    match package {
        Some(pkg) if is_package(pkg, CLAUDE_PACKAGE) => Detected::Kind(SessionKind::Claude),
        Some(pkg) if is_package(pkg, GEMINI_PACKAGE) => Detected::Kind(SessionKind::Gemini),
        _ => Detected::Wrapper(wrapper.to_string()),
    }
}

fn is_package(token: &str, package: &str) -> bool {
    token.strip_prefix(package).is_some_and(|rest| rest.is_empty() || rest.starts_with('@'))
}

/// 第一個不是選項（不以 `-` 開頭）的參數。
fn first_positional<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    tokens.find(|t| !t.starts_with('-'))
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("fixtures/ps_sample.txt");
    const CUE_PID: i32 = 50000;

    fn sample() -> Vec<ProcInfo> {
        parse_ps(SAMPLE, CUE_PID)
    }

    fn fg(tty: &str) -> Foreground {
        foreground_by_tty(&sample()).remove(tty).unwrap_or_else(|| panic!("{tty} 應有前景程式"))
    }

    fn kind_of(args: &str) -> SessionKind {
        let p = ProcInfo { pid: 1, ppid: 0, pgid: 1, tpgid: 1, tty: "ttys000".into(), args: args.into() };
        classify(&p, &[&p])
    }

    fn other(name: &str) -> SessionKind {
        SessionKind::Other(name.into())
    }

    #[test]
    fn parses_fields_and_keeps_args_with_spaces() {
        let procs = sample();
        let p = procs.iter().find(|p| p.pid == 512).unwrap();
        assert_eq!((p.ppid, p.pgid, p.tpgid), (511, 510, 510));
        assert_eq!(p.tty, "ttys004");
        assert_eq!(
            p.args,
            "/opt/homebrew/Cellar/node/24.4.1/bin/node --max-old-space-size=8192 /Users/user/.npm/_npx/6913fdfd1ea7a741/node_modules/.bin/gemini"
        );
        assert_eq!(procs.iter().find(|p| p.pid == 1500).unwrap().tty, "ttys1024");
    }

    #[test]
    fn skips_processes_without_tty_and_cue_children() {
        let procs = sample();
        assert!(procs.iter().all(|p| p.tty.starts_with("ttys")));
        assert!(!procs.iter().any(|p| [1, 330, 98].contains(&p.pid)), "?? 的程序應略過");
        assert!(!procs.iter().any(|p| p.ppid == CUE_PID), "Cue 的子程序應略過");
        assert!(procs.iter().any(|p| p.pid == CUE_PID), "Cue 本身不是自己的子程序");
    }

    #[test]
    fn skips_malformed_lines() {
        let text = "\n  abc 1 1 1 ttys000 vim\n  10 1 10 10 ttys000\n  11 1 11 11 ttys000   \n  12 1 12 12 ttys000 vim\n";
        let procs = parse_ps(text, CUE_PID);
        assert_eq!(procs.len(), 1);
        assert_eq!(procs[0].pid, 12);
    }

    #[test]
    fn foreground_is_group_leader_not_children() {
        // 原生 claude：MCP 子程序在同一群組，但前景只取領頭。
        assert_eq!(fg("ttys001"), Foreground { pid: 210, kind: SessionKind::Claude });
        // npm exec 包裝：領頭是 npm，類型由套件判斷，pid 仍是領頭。
        assert_eq!(fg("ttys003"), Foreground { pid: 410, kind: SessionKind::Claude });
        assert_eq!(fg("ttys004"), Foreground { pid: 510, kind: SessionKind::Gemini });
    }

    #[test]
    fn foreground_ignores_background_groups_on_same_tty() {
        // ttys002 同時有前景 vim 與背景 sleep，只取前景群組。
        assert_eq!(fg("ttys002"), Foreground { pid: 310, kind: other("vim") });
    }

    #[test]
    fn foreground_when_leader_exited_takes_smallest_pid() {
        assert_eq!(fg("ttys009"), Foreground { pid: 1011, kind: other("grep") });
    }

    #[test]
    fn foreground_covers_every_tty_in_sample() {
        let map = foreground_by_tty(&sample());
        let expected = [
            ("ttys000", 100, SessionKind::Zsh),
            ("ttys005", 610, SessionKind::Gemini),
            ("ttys006", 710, SessionKind::Claude),
            ("ttys007", 800, SessionKind::Zsh),
            ("ttys008", 910, other("npm")),
            ("ttys010", 1110, other("node")),
            ("ttys011", 1200, SessionKind::Zsh),
            ("ttys012", 1310, SessionKind::Claude),
            ("ttys013", 1410, other("npx")),
            ("ttys1024", 1500, SessionKind::Zsh),
        ];
        for (tty, pid, kind) in expected {
            assert_eq!(map.get(tty), Some(&Foreground { pid, kind }), "{tty}");
        }
        assert_eq!(map.len(), 15);
    }

    #[test]
    fn classify_claude_forms() {
        assert_eq!(kind_of("claude"), SessionKind::Claude);
        assert_eq!(kind_of("/Users/user/.local/bin/claude --resume"), SessionKind::Claude);
        assert_eq!(
            kind_of("node /opt/homebrew/lib/node_modules/@anthropic-ai/claude-code/cli.js"),
            SessionKind::Claude
        );
        assert_eq!(
            kind_of("bun /Users/user/.bun/install/global/node_modules/@anthropic-ai/claude-code/cli.js"),
            SessionKind::Claude
        );
        assert_eq!(kind_of("npm exec @anthropic-ai/claude-code"), SessionKind::Claude);
        assert_eq!(kind_of("npm exec --yes -- @anthropic-ai/claude-code@latest"), SessionKind::Claude);
        assert_eq!(kind_of("npx @anthropic-ai/claude-code"), SessionKind::Claude);
    }

    #[test]
    fn classify_gemini_forms() {
        assert_eq!(kind_of("node /Users/user/.npm/_npx/abc/node_modules/.bin/gemini"), SessionKind::Gemini);
        assert_eq!(
            kind_of("/opt/homebrew/bin/node --max-old-space-size=8192 /opt/homebrew/bin/gemini"),
            SessionKind::Gemini
        );
        assert_eq!(kind_of("npm exec @google/gemini-cli"), SessionKind::Gemini);
    }

    #[test]
    fn classify_shells() {
        for args in ["-zsh", "zsh", "/opt/homebrew/bin/zsh -l", "-bash", "/bin/bash", "fish", "/bin/sh"] {
            assert_eq!(kind_of(args), SessionKind::Zsh, "{args}");
        }
    }

    #[test]
    fn classify_others() {
        assert_eq!(kind_of("vim src/main.rs"), other("vim"));
        assert_eq!(kind_of("/usr/bin/vim"), other("vim"));
        assert_eq!(kind_of("npm run dev"), other("npm"));
        assert_eq!(kind_of("npm exec @anthropic-ai/claude-code-router"), other("npm"));
        assert_eq!(kind_of("node server.js"), other("node"));
        assert_eq!(kind_of("node -e \\012const f = 1"), other("node"));
        assert_eq!(kind_of("zshrc-helper"), other("zshrc-helper"));
    }

    #[test]
    fn classify_wrapper_searches_descendants_in_group() {
        let procs = sample();
        let group: Vec<&ProcInfo> = procs.iter().filter(|p| p.pgid == 510).collect();
        // 即使領頭的套件無法辨識，也會在子孫中找到 gemini。
        let leader = ProcInfo { args: "npx some-launcher".into(), ..group[0].clone() };
        assert_eq!(classify(&leader, &group), SessionKind::Gemini);
        // 子孫中沒有 claude／gemini 時退回領頭的執行檔名稱。
        let group: Vec<&ProcInfo> = procs.iter().filter(|p| p.pgid == 1410).collect();
        assert_eq!(classify(group[0], &group), other("npx"));
    }

    #[test]
    fn classify_does_not_descend_from_non_wrapper() {
        // 前景是 vim 時，即使群組裡有 claude 也不改判。
        let vim = ProcInfo { pid: 1, ppid: 0, pgid: 1, tpgid: 1, tty: "ttys000".into(), args: "vim".into() };
        let child = ProcInfo { pid: 2, ppid: 1, args: "claude".into(), ..vim.clone() };
        assert_eq!(classify(&vim, &[&vim, &child]), other("vim"));
    }

    #[test]
    #[ignore = "需要實機 ps"]
    fn snapshot_runs_real_ps() {
        let procs = snapshot().expect("ps 應可執行");
        let me = i32::try_from(std::process::id()).unwrap();
        assert!(!procs.iter().any(|p| p.ppid == me));
    }
}

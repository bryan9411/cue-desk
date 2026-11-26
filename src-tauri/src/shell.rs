//! 送進終端機的文字的**唯一建構點**。
//!
//! 其他模組不得自行組出要送進終端機的指令字串：路徑一律經 [`quote`] 跳脫，
//! 指令名稱只能來自 [`AiCommand`] 固定列舉，且不提供附加參數的介面
//! （跳過權限確認的參數永遠不會出現在程式中）。

use std::path::Path;

use serde::Deserialize;

use crate::error::CueError;

/// 以單引號包住字串，內部的 `'` 轉為 `'\''`。
///
/// 含換行（`\n`、`\r`）或任何其他控制字元（`\0`、`\x1b`、`\x7f` 等）時回傳
/// [`CueError::UnsafePath`]：這些字元送進終端機可能提前執行指令或被當成按鍵。
pub fn quote(s: &str) -> Result<String, CueError> {
    if s.chars().any(char::is_control) {
        return Err(CueError::UnsafePath);
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    Ok(out)
}

/// 允許在終端機中啟動的 AI CLI。只有固定的指令名稱，不接受任意字串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiCommand {
    Claude,
    Gemini,
}

impl AiCommand {
    pub fn as_str(self) -> &'static str {
        match self {
            AiCommand::Claude => "claude",
            AiCommand::Gemini => "gemini",
        }
    }
}

/// 建立工作階段時可選的類型。前端只能送 `"claude"`／`"gemini"`／`"zsh"`，其他值反序列化失敗。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionKindToCreate {
    Claude,
    Gemini,
    Zsh,
}

impl SessionKindToCreate {
    /// 要啟動的 AI CLI；`Zsh` 為 `None`。
    pub fn ai(self) -> Option<AiCommand> {
        match self {
            SessionKindToCreate::Claude => Some(AiCommand::Claude),
            SessionKindToCreate::Gemini => Some(AiCommand::Gemini),
            SessionKindToCreate::Zsh => None,
        }
    }
}

/// 切換到工作區的指令：`cd -- '<路徑>'`。不是合法 UTF-8 的路徑視為不安全。
pub fn cd_command(workspace: &Path) -> Result<String, CueError> {
    let path = workspace.to_str().ok_or(CueError::UnsafePath)?;
    Ok(format!("cd -- {}", quote(path)?))
}

/// 建立工作階段時送入新窗格的指令。
///
/// - `Claude` → `cd -- '<路徑>' && claude`
/// - `Gemini` → `cd -- '<路徑>' && gemini`
/// - `Zsh` → `cd -- '<路徑>'`
pub fn launch_command(kind: SessionKindToCreate, workspace: &Path) -> Result<String, CueError> {
    let cd = cd_command(workspace)?;
    Ok(match kind.ai() {
        Some(ai) => format!("{cd} && {}", ai.as_str()),
        None => cd,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    #[test]
    fn quotes_plain_and_special_strings() {
        assert_eq!(quote("/Users/me/proj").unwrap(), "'/Users/me/proj'");
        assert_eq!(quote("").unwrap(), "''");
        assert_eq!(quote("A's").unwrap(), "'A'\\''s'");
        assert_eq!(quote("''").unwrap(), "''\\'''\\'''");
        // 單引號內 $、反引號、雙引號、反斜線都沒有特殊意義，原樣保留。
        assert_eq!(quote("$(touch /tmp/x)").unwrap(), "'$(touch /tmp/x)'");
        assert_eq!(quote("`id`").unwrap(), "'`id`'");
        assert_eq!(quote("say \"x\"").unwrap(), "'say \"x\"'");
        assert_eq!(quote("a\\b").unwrap(), "'a\\b'");
        assert_eq!(quote("我的 專案 2").unwrap(), "'我的 專案 2'");
    }

    #[test]
    fn rejects_control_characters() {
        for s in ["a\nb", "a\rb", "a\0b", "a\x1b[31mb", "a\x7fb", "a\tb", "\u{85}"] {
            assert_eq!(quote(s), Err(CueError::UnsafePath), "應拒絕 {s:?}");
        }
        assert_eq!(
            launch_command(SessionKindToCreate::Claude, Path::new("/tmp/a\nrm -rf ~")),
            Err(CueError::UnsafePath)
        );
    }

    #[test]
    fn builds_launch_commands() {
        let ws = Path::new("/Users/me/A's 專案");
        assert_eq!(
            launch_command(SessionKindToCreate::Claude, ws).unwrap(),
            "cd -- '/Users/me/A'\\''s 專案' && claude"
        );
        assert_eq!(
            launch_command(SessionKindToCreate::Gemini, ws).unwrap(),
            "cd -- '/Users/me/A'\\''s 專案' && gemini"
        );
        assert_eq!(
            launch_command(SessionKindToCreate::Zsh, ws).unwrap(),
            "cd -- '/Users/me/A'\\''s 專案'"
        );
    }

    #[test]
    fn ai_commands_are_fixed() {
        assert_eq!(AiCommand::Claude.as_str(), "claude");
        assert_eq!(AiCommand::Gemini.as_str(), "gemini");
        assert_eq!(SessionKindToCreate::Zsh.ai(), None);
    }

    #[test]
    fn kind_to_create_accepts_only_known_values() {
        let parse = |s: &str| serde_json::from_value::<SessionKindToCreate>(serde_json::json!(s));
        assert_eq!(parse("claude").unwrap(), SessionKindToCreate::Claude);
        assert_eq!(parse("gemini").unwrap(), SessionKindToCreate::Gemini);
        assert_eq!(parse("zsh").unwrap(), SessionKindToCreate::Zsh);
        for bad in ["bash", "Claude", "claude --yolo", "claude --dangerously-skip-permissions", ""] {
            assert!(parse(bad).is_err(), "應拒絕 {bad:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_is_rejected() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let p = PathBuf::from(OsStr::from_bytes(b"/tmp/\xff"));
        assert_eq!(cd_command(&p), Err(CueError::UnsafePath));
    }

    /// 實際交給 zsh 執行：路徑中的 `$(…)` 不會被執行，`cd` 進入正確目錄。
    #[test]
    fn zsh_does_not_execute_injected_path() {
        let pwned = Path::new("/tmp/pwned");
        let _ = std::fs::remove_file(pwned);
        assert!(!pwned.exists(), "測試前無法移除 /tmp/pwned");

        let tmp = tempfile::tempdir().unwrap();
        // 名稱中的 `/` 讓它成為多層目錄，但整段路徑字串仍含完整的注入內容。
        let ws = tmp.path().join("A's 專案 $(touch /tmp/pwned) \"x\" `touch /tmp/pwned`");
        std::fs::create_dir_all(&ws).unwrap();

        let cmd = format!("{} && pwd -P", launch_command(SessionKindToCreate::Zsh, &ws).unwrap());
        let out = Command::new("/bin/zsh").arg("-f").arg("-c").arg(&cmd).output().unwrap();
        assert!(out.status.success(), "zsh 失敗：{}", String::from_utf8_lossy(&out.stderr));

        let printed = String::from_utf8(out.stdout).unwrap();
        let expected = ws.canonicalize().unwrap();
        assert_eq!(printed.trim_end_matches('\n'), expected.to_str().unwrap());
        assert!(!pwned.exists(), "注入的指令被執行了");
    }
}

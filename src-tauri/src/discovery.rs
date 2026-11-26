//! 探索迴圈：每 2 秒探索一次終端機工作階段，合併進登錄，有變動時通知前端。
//!
//! - 每輪在 `spawn_blocking` 中執行（osascript、`ps` 都是阻塞呼叫）；一輪結束後才排下一輪，不重疊。
//! - 終端機未執行時不跑任何腳本（**不得為了輪詢把終端機開起來**），只把先前存活的節點標為 `Exited`。
//! - 探索失敗（例如未取得自動化權限）時不合併，避免把所有節點誤判為已結束。

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{interval, MissedTickBehavior};

use crate::error::CueError;
use crate::model::DiscoveredSession;
use crate::process::{self, Foreground};
use crate::registry::SessionNode;
use crate::state::{lock, workspace_roots, AppState};

/// 輪詢間隔。
pub const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// 單輪超過此耗時時記錄警告，作為調整輪詢間隔的依據。
const SLOW_ROUND: Duration = Duration::from_secs(1);
/// 快照有變動時送給前端的事件。
pub const SESSIONS_UPDATED: &str = "sessions-updated";

/// 只在開發版輸出的除錯紀錄。
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*);
        }
    };
}

/// 啟動背景探索迴圈。需在 `AppState` 註冊之後呼叫。
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(run_loop(app));
}

async fn run_loop(app: AppHandle) {
    let mut ticker = interval(POLL_INTERVAL);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_error: Option<CueError> = None;

    loop {
        {
            let state = app.state::<AppState>();
            tokio::select! {
                _ = ticker.tick() => {}
                _ = state.discovery_trigger.notified() => {}
            }
        }

        let started = Instant::now();
        let handle = app.clone();
        let result =
            tauri::async_runtime::spawn_blocking(move || run_round(&handle.state::<AppState>())).await;
        let elapsed = started.elapsed();

        match result {
            Ok(Ok(count)) => {
                if let Some(e) = last_error.take() {
                    eprintln!("[cue] 探索已恢復（先前錯誤：{e}）");
                }
                debug_log!("[cue] 探索 {} ms，{count} 個工作階段", elapsed.as_millis());
            }
            Ok(Err(e)) => {
                // 同一個錯誤只記錄一次，避免每 2 秒洗版。
                if last_error.as_ref() != Some(&e) {
                    eprintln!("[cue] 探索失敗：{e}");
                }
                last_error = Some(e);
            }
            Err(e) => eprintln!("[cue] 探索工作異常結束：{e}"),
        }
        if elapsed > SLOW_ROUND {
            eprintln!("[cue] 探索耗時 {} ms，超過 1 秒", elapsed.as_millis());
        }

        publish(&app);
        // 從本輪結束起算下一輪，立即觸發的那輪也不會緊接著再跑一次。
        ticker.reset();
    }
}

/// 終端機與程序資訊的一次取樣。
struct Sample {
    discovered: Vec<DiscoveredSession>,
    foregrounds: HashMap<String, Foreground>,
    cwds: HashMap<i32, PathBuf>,
}

/// 取樣（不持有任何鎖）：探索終端機 → 一次 `ps` → 只對探索到的 tty 批次取 cwd。
fn sample(state: &AppState) -> Result<Sample, CueError> {
    let backend = state.backend();
    if !backend.is_running() {
        return Ok(Sample { discovered: Vec::new(), foregrounds: HashMap::new(), cwds: HashMap::new() });
    }
    let discovered = backend.discover()?;
    let procs = process::snapshot()?;
    let mut foregrounds = process::foreground_by_tty(&procs);
    // 前景程式表只用來查詢探索到的 tty，不為其他 tty 產生工作階段。
    foregrounds.retain(|tty, _| discovered.iter().any(|d| &d.tty == tty));
    let pids: Vec<i32> = foregrounds.values().map(|f| f.pid).collect();
    let cwds = process::cwds(&pids);
    Ok(Sample { discovered, foregrounds, cwds })
}

/// 執行一輪探索並合併，回傳存活的工作階段數。
fn run_round(state: &AppState) -> Result<usize, CueError> {
    let s = sample(state)?;

    let mut config = lock(&state.config);
    let roots = workspace_roots(&config);
    let mut registry = lock(&state.registry);
    let outcome = registry.merge(&s.discovered, &s.foregrounds, &s.cwds, &roots, &mut config);
    if outcome.config_changed {
        state.save_config(&config);
    }
    if outcome.nodes_changed && cfg!(debug_assertions) {
        log_summary(registry.nodes());
    }
    Ok(registry.nodes().iter().filter(|n| n.alive).count())
}

fn log_summary(nodes: &[SessionNode]) {
    eprintln!("[cue] 節點變動，共 {} 個：", nodes.len());
    for n in nodes {
        eprintln!(
            "[cue]   {} {} [{}] {:?} → {}",
            n.id,
            n.name(),
            n.location_hint,
            n.status,
            n.workspace_id.as_deref().unwrap_or("未歸類"),
        );
    }
}

/// 快照與上次送出的不同時才送出 `sessions-updated`。探索迴圈與會改變狀態的指令共用。
pub fn publish(app: &AppHandle) {
    let state = app.state::<AppState>();
    let snapshot = state.snapshot();
    let mut last = lock(&state.last_emitted);
    if last.as_ref() == Some(&snapshot) {
        return;
    }
    if let Err(e) = app.emit(SESSIONS_UPDATED, &snapshot) {
        eprintln!("[cue] 無法送出 {SESSIONS_UPDATED}：{e}");
        return;
    }
    *last = Some(snapshot);
}

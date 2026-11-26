//! `~/.cue/config.json` 設定檔：資料模型、讀取與原子寫入。
//!
//! - 所有欄位都有預設值；未知欄位（頂層與工作區）原樣保留，避免新舊版本互相覆蓋時遺失資料。
//! - 寫入一律「寫暫存檔 → `sync_all` → `rename`」，並經 [`ConfigWriter`] 以 debounce 合併。
//! - 讀取失敗（JSON 壞掉）時把原檔備份為 `config.json.bak-<毫秒時間戳>`，以預設值啟動。

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::model::SessionKind;

/// 目前的設定檔格式版本。
pub const CONFIG_VERSION: u32 = 1;
/// 寫檔合併的預設等待時間。
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_millis(300);

const CONFIG_DIR_NAME: &str = ".cue";
const CONFIG_FILE_NAME: &str = "config.json";

/// 設定檔根結構。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CueConfig {
    pub version: u32,
    pub workspaces: Vec<Workspace>,
    /// 以 `SessionKey` 字串為鍵的節點資訊（標籤與最後已知狀態）。
    pub nodes: HashMap<String, NodeMeta>,
    /// 未歸類區的忽略清單（`SessionKey` 字串）。
    pub ignored: Vec<String>,
    pub terminal_backend: TerminalBackendPref,
    /// 偏好設定；之後再定型別，先原樣保留。
    pub preferences: Map<String, Value>,
    /// 視窗位置與大小；之後再定型別，先原樣保留。
    pub window: Map<String, Value>,
    /// 目前版本不認得的頂層欄位，原樣保留。
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for CueConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            workspaces: Vec::new(),
            nodes: HashMap::new(),
            ignored: Vec::new(),
            terminal_backend: TerminalBackendPref::default(),
            preferences: Map::new(),
            window: Map::new(),
            extra: Map::new(),
        }
    }
}

impl CueConfig {
    /// 補齊讀入後缺漏的衍生欄位（例如工作區名稱）。
    fn normalize(&mut self) {
        for ws in &mut self.workspaces {
            if ws.name.trim().is_empty() {
                ws.name = default_workspace_name(&ws.path);
            }
        }
    }

    /// 產生一個不與既有工作區重複的 id。
    pub fn new_workspace_id(&self) -> String {
        let mut n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        loop {
            let id = format!("ws-{n:x}");
            if !self.workspaces.iter().any(|w| w.id == id) {
                return id;
            }
            n = n.wrapping_add(1);
        }
    }
}

/// 已加入的工作區。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    /// realpath 後的根目錄。
    pub path: PathBuf,
    /// 顯示名稱，預設為資料夾名稱。
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub collapsed: bool,
    /// 保留給未來的擴充欄位，讀寫時原樣保留。
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Workspace {
    pub fn new(id: String, path: PathBuf) -> Self {
        let name = default_workspace_name(&path);
        Self { id, path, name, collapsed: false, extra: Map::new() }
    }
}

/// 資料夾名稱；根目錄等沒有名稱的路徑以完整路徑代替。
fn default_workspace_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// 依節點身分記錄的資訊。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeMeta {
    pub label: Option<String>,
    pub last_tty: Option<String>,
    pub last_cwd: Option<PathBuf>,
    pub last_kind: Option<SessionKind>,
}

/// 使用的終端機後端。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TerminalBackendPref {
    /// 依安裝與執行狀態自動選擇。
    #[default]
    Auto,
    Iterm2,
    Terminal,
}

impl TerminalBackendPref {
    pub fn as_str(self) -> &'static str {
        match self {
            TerminalBackendPref::Auto => "auto",
            TerminalBackendPref::Iterm2 => "iterm2",
            TerminalBackendPref::Terminal => "terminal",
        }
    }
}

impl Serialize for TerminalBackendPref {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// 不認得的值一律視為 `auto`：使用者手動編輯打錯字時，不應讓整份設定被當成壞檔。
impl<'de> Deserialize<'de> for TerminalBackendPref {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(deserializer)?;
        Ok(match v.as_str() {
            Some("iterm2") => TerminalBackendPref::Iterm2,
            Some("terminal") => TerminalBackendPref::Terminal,
            _ => TerminalBackendPref::Auto,
        })
    }
}

/// `~/.cue/config.json`；取不到 `HOME` 時回傳 `None`。
pub fn default_config_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty())?;
    Some(PathBuf::from(home).join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME))
}

/// 讀取設定檔。檔案不存在時回傳預設值；內容無法解析時先備份原檔再回傳預設值。
pub fn load(path: &Path) -> CueConfig {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return CueConfig::default(),
        Err(e) => {
            eprintln!("[cue] 無法讀取設定檔 {}：{e}", path.display());
            return CueConfig::default();
        }
    };
    match serde_json::from_str::<CueConfig>(&text) {
        Ok(mut config) => {
            config.normalize();
            config
        }
        Err(e) => {
            eprintln!("[cue] 設定檔格式錯誤，改用預設值：{e}");
            match backup_broken(path) {
                Ok(bak) => eprintln!("[cue] 原設定檔已備份為 {}", bak.display()),
                Err(e) => eprintln!("[cue] 無法備份設定檔：{e}"),
            }
            CueConfig::default()
        }
    }
}

/// 把壞掉的設定檔改名為 `config.json.bak-<毫秒時間戳>`。
fn backup_broken(path: &Path) -> io::Result<PathBuf> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bak-{millis}"));
    let bak = path.with_file_name(name);
    fs::rename(path, &bak)?;
    Ok(bak)
}

/// 原子寫入：同目錄寫 `<檔名>.tmp` → `sync_all` → `rename`。目錄權限設為 `0700`、檔案 `0600`。
pub fn save_atomic(path: &Path, config: &CueConfig) -> io::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "設定檔路徑沒有上層目錄"))?;
    fs::create_dir_all(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;

    let mut json = serde_json::to_vec_pretty(config).map_err(io::Error::other)?;
    json.push(b'\n');

    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);

    let result = (|| {
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        file.write_all(&json)?;
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

enum WriterMsg {
    Save(Box<CueConfig>),
    Flush(mpsc::SyncSender<()>),
}

/// 背景寫檔器：同一段時間內的多次 [`save`](ConfigWriter::save) 只寫入最後一版。
///
/// 第一次變動後最多等待 `debounce` 就寫檔（不會因持續變動而一直延後）。
/// 寫入失敗只記錄錯誤，不影響 App。被 drop 時會先寫完尚未寫入的內容再結束執行緒。
pub struct ConfigWriter {
    tx: mpsc::Sender<WriterMsg>,
}

impl ConfigWriter {
    pub fn spawn(path: PathBuf, debounce: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("cue-config-writer".into())
            .spawn(move || writer_loop(rx, &path, debounce))
            .expect("無法啟動設定檔寫入執行緒");
        Self { tx }
    }

    /// 排入一次寫檔。
    pub fn save(&self, config: CueConfig) {
        let _ = self.tx.send(WriterMsg::Save(Box::new(config)));
    }

    /// 立即寫入尚未寫入的內容並等待完成（例如 App 結束前）。
    pub fn flush(&self) {
        let (ack_tx, ack_rx) = mpsc::sync_channel(1);
        if self.tx.send(WriterMsg::Flush(ack_tx)).is_ok() {
            let _ = ack_rx.recv();
        }
    }
}

fn writer_loop(rx: mpsc::Receiver<WriterMsg>, path: &Path, debounce: Duration) {
    let mut pending: Option<(Box<CueConfig>, Instant)> = None;
    let write = |config: &CueConfig| {
        if let Err(e) = save_atomic(path, config) {
            eprintln!("[cue] 寫入設定檔失敗：{e}");
        }
    };
    loop {
        let msg = match &pending {
            None => match rx.recv() {
                Ok(m) => m,
                Err(_) => return,
            },
            Some((_, deadline)) => {
                match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                    Ok(m) => m,
                    Err(RecvTimeoutError::Timeout) => {
                        if let Some((c, _)) = pending.take() {
                            write(&c);
                        }
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        if let Some((c, _)) = pending.take() {
                            write(&c);
                        }
                        return;
                    }
                }
            }
        };
        match msg {
            WriterMsg::Save(c) => {
                let deadline = pending.take().map(|(_, d)| d).unwrap_or_else(|| Instant::now() + debounce);
                pending = Some((c, deadline));
            }
            WriterMsg::Flush(ack) => {
                if let Some((c, _)) = pending.take() {
                    write(&c);
                }
                let _ = ack.send(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_config() -> CueConfig {
        let mut c = CueConfig::default();
        c.workspaces.push(Workspace::new("ws-1".into(), PathBuf::from("/Users/u/code/cue")));
        c.nodes.insert(
            "iterm2:ABC-123".into(),
            NodeMeta {
                label: Some("後端".into()),
                last_tty: Some("ttys003".into()),
                last_cwd: Some(PathBuf::from("/Users/u/code/cue")),
                last_kind: Some(SessionKind::Claude),
            },
        );
        c.ignored.push("terminal:42:ttys001".into());
        c.terminal_backend = TerminalBackendPref::Terminal;
        c.preferences.insert("fontSize".into(), json!(13));
        c
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> =
            fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        v.sort();
        v
    }

    #[test]
    fn round_trip_serialization() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let config = sample_config();
        save_atomic(&path, &config).unwrap();
        assert_eq!(load(&path), config);

        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["version"], json!(1));
        assert_eq!(v["terminalBackend"], json!("terminal"));
        assert_eq!(v["nodes"]["iterm2:ABC-123"]["lastTty"], json!("ttys003"));
        assert_eq!(v["nodes"]["iterm2:ABC-123"]["lastKind"], json!({ "type": "claude" }));
        assert_eq!(v["workspaces"][0]["name"], json!("cue"));
    }

    #[test]
    fn missing_file_and_fields_use_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        assert_eq!(load(&path), CueConfig::default());

        fs::write(&path, r#"{ "workspaces": [{ "id": "a", "path": "/tmp/proj" }] }"#).unwrap();
        let c = load(&path);
        assert_eq!(c.version, CONFIG_VERSION);
        assert_eq!(c.terminal_backend, TerminalBackendPref::Auto);
        assert_eq!(c.workspaces[0].name, "proj");
        assert!(!c.workspaces[0].collapsed);
    }

    #[test]
    fn unknown_fields_are_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(
            &path,
            r#"{
                "workspaces": [{ "id": "a", "path": "/p", "name": "P", "color": "teal", "pinned": true }],
                "window": { "x": 10 },
                "futureTopLevel": [1, 2]
            }"#,
        )
        .unwrap();
        let c = load(&path);
        assert_eq!(c.workspaces[0].extra.get("color"), Some(&json!("teal")));
        save_atomic(&path, &c).unwrap();

        let v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["workspaces"][0]["color"], json!("teal"));
        assert_eq!(v["workspaces"][0]["pinned"], json!(true));
        assert_eq!(v["window"], json!({ "x": 10 }));
        assert_eq!(v["futureTopLevel"], json!([1, 2]));
    }

    #[test]
    fn unknown_backend_falls_back_to_auto() {
        let c: CueConfig = serde_json::from_str(r#"{ "terminalBackend": "warp" }"#).unwrap();
        assert_eq!(c.terminal_backend, TerminalBackendPref::Auto);
        let c: CueConfig = serde_json::from_str(r#"{ "terminalBackend": "iterm2" }"#).unwrap();
        assert_eq!(c.terminal_backend, TerminalBackendPref::Iterm2);
    }

    #[test]
    fn broken_file_is_backed_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, "{ not json").unwrap();

        assert_eq!(load(&path), CueConfig::default());
        let names = entries(dir.path());
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].starts_with("config.json.bak-"), "{names:?}");
        assert_eq!(fs::read_to_string(dir.path().join(&names[0])).unwrap(), "{ not json");
    }

    #[test]
    fn atomic_write_leaves_no_tmp_and_sets_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let cue_dir = dir.path().join(".cue");
        let path = cue_dir.join("config.json");
        save_atomic(&path, &sample_config()).unwrap();
        save_atomic(&path, &CueConfig::default()).unwrap();

        assert_eq!(entries(&cue_dir), vec!["config.json".to_string()]);
        assert_eq!(fs::metadata(&cue_dir).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(load(&path), CueConfig::default());
    }

    #[test]
    fn writer_debounces_to_latest_and_flushes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let writer = ConfigWriter::spawn(path.clone(), Duration::from_secs(60));

        for i in 0..5 {
            let mut c = CueConfig::default();
            c.ignored.push(format!("iterm2:N{i}"));
            writer.save(c);
        }
        assert!(!path.exists(), "debounce 期間不應寫檔");

        writer.flush();
        assert_eq!(load(&path).ignored, vec!["iterm2:N4".to_string()]);
        assert_eq!(entries(dir.path()), vec!["config.json".to_string()]);
    }

    #[test]
    fn writer_writes_after_debounce_and_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let writer = ConfigWriter::spawn(path.clone(), Duration::from_millis(20));
        writer.save(sample_config());
        let start = Instant::now();
        while !path.exists() && start.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(load(&path), sample_config());

        let writer2 = ConfigWriter::spawn(path.clone(), Duration::from_secs(60));
        writer2.save(CueConfig::default());
        drop(writer);
        drop(writer2);
        let start = Instant::now();
        while load(&path) != CueConfig::default() && start.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(load(&path), CueConfig::default());
    }

    #[test]
    fn new_workspace_id_is_unique() {
        let mut c = CueConfig::default();
        for _ in 0..50 {
            let id = c.new_workspace_id();
            assert!(id.starts_with("ws-"));
            c.workspaces.push(Workspace::new(id, PathBuf::from("/p")));
        }
        let mut ids: Vec<_> = c.workspaces.iter().map(|w| w.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 50);
    }
}

# 技術探針實機紀錄

> 紀錄日期：2026-11-26
> 環境：macOS 26.6.2、iTerm2 3.7.8、原生 Claude Code 2.1.332、npm `@anthropic-ai/claude-code` 2.1.332、`@google/gemini-cli` 0.71.1（後兩者以 `npx` 臨時下載，測完已刪除）。
> 探針：`probe_discover`、`probe_cwd`（`cargo run --manifest-path src-tauri/Cargo.toml --example <名稱>`）。
> 共兩輪：第一輪為使用者日常環境（1 個 iTerm2 視窗、7 個分頁），第二輪依 1-2-H1 情境以 AppleScript 建立測試視窗後執行。

## 情境涵蓋狀況

| # | 情境 | 結果 |
|---|---|---|
| 1 | 兩個獨立 iTerm2 視窗 `cd` 到同一資料夾 | 已測：視窗 11896 與 9782 都有 `bryan-log` 分頁，各自獨立一筆 |
| 2 | 同一分頁左右、上下分割 | 已測：三個窗格各自一筆，`tab#` 相同 |
| 3 | 原生 `claude`、npm `claude`、`gemini` | 已測（npm 版以 `npx` 執行） |
| 4 | 一個分頁執行 `vim` | 已測 |
| 5 | 結束 iTerm2 後執行 `probe_discover` | 已測：不會把 iTerm2 打開 |
| 6 | Terminal.app 兩個視窗、其一有兩個分頁 | 已測：**新分頁被回報為獨立視窗**（見下） |
| — | 最小化視窗 | 已測（視窗 11905） |
| — | 全螢幕視窗 | 未測（會切換使用者的桌面空間，略過） |

---

## 第一輪：日常環境

### `probe_discover`

```
== iTerm2 (com.googlecode.iterm2) ==
執行前 is running: true
執行後 is running: true
耗時：#1 687.4 ms、#2 698.9 ms、#3 685.5 ms
win_id | win# | left | top | right | bottom | tab# | unique_id                            | tty          | name
9782   | 1    | 1586 | 82  | 3220  | 1018   | 1    | C524DA83-2B9D-4DD2-8D95-335F59CA2978 | /dev/ttys000 | ..nts/bryan-log (-zsh)
9782   | 1    | 1586 | 82  | 3220  | 1018   | 2    | 9CC3F129-8C80-49DE-845A-1DBC8576E2DE | /dev/ttys002 | ..nts/my-skills (-zsh)
9782   | 1    | 1586 | 82  | 3220  | 1018   | 3    | FA735C07-1483-4211-953C-1F72BB3E4F54 | /dev/ttys003 | ..nts/bryan-log (-zsh)
9782   | 1    | 1586 | 82  | 3220  | 1018   | 4    | B81259B0-2139-44A1-A126-71F35E8CC3DD | /dev/ttys005 | ~/.claude/hooks (-zsh)
9782   | 1    | 1586 | 82  | 3220  | 1018   | 5    | A5CE1159-3857-4CE2-AE41-DA19FDB95551 | /dev/ttys006 | ..uments/saidso (-zsh)
9782   | 1    | 1586 | 82  | 3220  | 1018   | 6    | 96F113DE-FEF6-442D-A134-5B081A88C6A9 | /dev/ttys007 | ~ (-zsh)
9782   | 1    | 1586 | 82  | 3220  | 1018   | 7    | 4DA90FAA-598D-412A-8219-C0C510C378E4 | /dev/ttys012 | cargo (probe_discover)
共 7 筆

== Terminal.app (com.apple.Terminal) ==
執行前 is running: false
執行後 is running: false
耗時：#1 62.9 ms、#2 49.5 ms、#3 62.5 ms
（沒有紀錄）
```

### `probe_cwd`

```
ps：582 支程序，前景 16 支（15 個 tty），耗時 69.2 ms
pid    tty      kind   args                                       libproc                               lsof                                  same
92232  ttys000  shell  -zsh                                       /Users/admin/Documents/bryan-log      /Users/admin/Documents/bryan-log      =
92262  ttys002  shell  -zsh                                       /Users/admin/Documents/my-skills      /Users/admin/Documents/my-skills      =
92278  ttys003  shell  -zsh                                       /Users/admin/Documents/bryan-log      /Users/admin/Documents/bryan-log      =
12629  ttys004  shell  /opt/homebrew/bin/zsh -l                   /Users/admin/Documents/triagent       /Users/admin/Documents/triagent       =
92310  ttys005  shell  -zsh                                       /Users/admin/.claude/hooks            /Users/admin/.claude/hooks            =
92330  ttys006  shell  -zsh                                       /Users/admin/Documents/saidso         /Users/admin/Documents/saidso         =
92372  ttys007  shell  -zsh                                       /Users/admin                          /Users/admin                          =
49135  ttys008  shell  /opt/homebrew/bin/zsh -l                   /Users/admin/Documents/bryan-log      /Users/admin/Documents/bryan-log      =
21225  ttys009  shell  /opt/homebrew/bin/zsh -l                   /Users/admin                          /Users/admin                          =
21780  ttys010  shell  /opt/homebrew/bin/zsh -l                   /Users/admin/Documents/claude-config  /Users/admin/Documents/claude-config  =
95110  ttys011  shell  /opt/homebrew/bin/zsh -l                   /Users/admin/Documents/geckolink      /Users/admin/Documents/geckolink      =
83505  ttys012  other  src-tauri/target/debug/examples/probe_cwd  /Users/admin/Documents/cue-desk       /Users/admin/Documents/cue-desk       =
83508  ttys012  other  /bin/ps -A -ww -o pid=,ppid=,pgid=,…        ERR No such process (os error 3)      ERR （無輸出）                         -
51994  ttys013  shell  /opt/homebrew/bin/zsh -l                   /Users/admin/Documents/cue-desk       /Users/admin/Documents/cue-desk       =
18649  ttys016  shell  /opt/homebrew/bin/zsh -l                   /Users/admin/Documents/triagent       /Users/admin/Documents/triagent       =
50465  ttys020  shell  /opt/homebrew/bin/zsh -l                   /Users/admin/Documents/saidso         /Users/admin/Documents/saidso         =

libproc：16 支 pid，各輪耗時 47 µs / 22 µs / 21 µs（平均每支 1 µs）
lsof   ：16 支 pid 一次執行，各輪耗時 7.8 ms / 4.7 ms / 8.7 ms

非本使用者程序（pid 1）：
  libproc：錯誤 Operation not permitted (os error 1)（errno 1）
  lsof   ：（無輸出）；stderr：（無）
```

---

## 第二輪：1-2-H1 情境

測試視窗：iTerm2 視窗 11896（分頁 1 做左右＋上下分割；分頁 2 原生 `claude`；分頁 3 `vim`；分頁 4 為誤觸 Cmd+T 產生的空白分頁；分頁 5 npm `claude`；分頁 6 `gemini`），視窗 11905（已最小化）。原有視窗 9782 不動。

### `probe_discover`（iTerm2 與 Terminal.app 都在執行）

```
== iTerm2 (com.googlecode.iterm2) ==
執行前 is running: true
執行後 is running: true
耗時：#1 1410.6 ms、#2 1395.1 ms、#3 1386.6 ms
win_id | win# | left | top | right | bottom | tab# | unique_id                            | tty          | name
11896  | 1    | 2649 | 495 | 3219  | 967    | 1    | 281FB514-2378-4265-804C-C17B1E9060AE | /dev/ttys001 | ..nts/bryan-log (-zsh)
11896  | 1    | 2649 | 495 | 3219  | 967    | 1    | F2769FF8-6FE7-4E82-8155-D4E41EC099A7 | /dev/ttys014 | ..nts/bryan-log (-zsh)
11896  | 1    | 2649 | 495 | 3219  | 967    | 1    | 436736AD-089F-482D-94C8-E1026897BB9F | /dev/ttys015 | ..ents/cue-desk (-zsh)
11896  | 1    | 2649 | 495 | 3219  | 967    | 2    | 5769FDC8-FDDF-4894-92FC-87F9B49676BF | /dev/ttys017 | ✳ Claude Code (claude)
11896  | 1    | 2649 | 495 | 3219  | 967    | 3    | C9AD4A45-A37D-40DC-9F69-B342C88EDD06 | /dev/ttys018 | cd (vim)
11896  | 1    | 2649 | 495 | 3219  | 967    | 4    | 5837333E-3C37-42DB-AFE6-CF7D27365D90 | /dev/ttys023 | ~ (-zsh)
11896  | 1    | 2649 | 495 | 3219  | 967    | 5    | AC5F161C-780F-45C6-B12E-65618E7389FB | /dev/ttys024 | ✳ Claude Code (claude)
11896  | 1    | 2649 | 495 | 3219  | 967    | 6    | 83B8BD38-9C66-4D57-B776-A1B2417649BB | /dev/ttys025 | ◇  Ready (cue-desk) (node)
9782   | 2    | 1586 | 82  | 3220  | 1018   | 1    | C524DA83-2B9D-4DD2-8D95-335F59CA2978 | /dev/ttys000 | ..nts/bryan-log (-zsh)
9782   | 2    | 1586 | 82  | 3220  | 1018   | 2    | 9CC3F129-8C80-49DE-845A-1DBC8576E2DE | /dev/ttys002 | ..nts/my-skills (-zsh)
9782   | 2    | 1586 | 82  | 3220  | 1018   | 3    | FA735C07-1483-4211-953C-1F72BB3E4F54 | /dev/ttys003 | ..nts/bryan-log (-zsh)
9782   | 2    | 1586 | 82  | 3220  | 1018   | 4    | B81259B0-2139-44A1-A126-71F35E8CC3DD | /dev/ttys005 | ~/.claude/hooks (-zsh)
9782   | 2    | 1586 | 82  | 3220  | 1018   | 5    | A5CE1159-3857-4CE2-AE41-DA19FDB95551 | /dev/ttys006 | ..uments/saidso (-zsh)
9782   | 2    | 1586 | 82  | 3220  | 1018   | 6    | 96F113DE-FEF6-442D-A134-5B081A88C6A9 | /dev/ttys007 | ~ (-zsh)
9782   | 2    | 1586 | 82  | 3220  | 1018   | 7    | 4DA90FAA-598D-412A-8219-C0C510C378E4 | /dev/ttys012 | ..ents/cue-desk (-zsh)
11905  | 3    | 1470 | 643 | 2040  | 1080   | 1    | B98AEF09-9608-451B-BF3D-77A455C336F4 | /dev/ttys019 | ..ents/cue-desk (-zsh)
共 16 筆

== Terminal.app (com.apple.Terminal) ==
執行前 is running: true
執行後 is running: true
耗時：#1 406.4 ms、#2 397.0 ms、#3 406.9 ms
win_id | win# | left | top | right | bottom | tab# | tty          | custom_title
11955  | 1    | 1627 | 119 | 2207  | 540    | 1    | /dev/ttys026 | admin@admindeMacBook-Air
11917  | 2    | 1470 | 68  | 2050  | 453    | 1    | /dev/ttys021 | admin@admindeMacBook-Air
11920  | 3    | 1627 | 119 | 2207  | 540    | 1    | /dev/ttys022 | admin@admindeMacBook-Air
共 3 筆
```

Terminal.app 在按 Cmd+T 之前（兩個視窗各一個分頁）：

```
耗時：#1 294.8 ms、#2 326.6 ms、#3 313.9 ms
11920  | 1    | 1499 | 97  | 2079  | 482    | 1    | /dev/ttys022 | admin@admindeMacBook-Air
11917  | 2    | 1470 | 68  | 2050  | 453    | 1    | /dev/ttys021 | admin@admindeMacBook-Air
```

### `probe_discover`（結束 iTerm2 後）

```
== iTerm2 (com.googlecode.iterm2) ==
執行前 is running: false
執行後 is running: false
耗時：#1 72.9 ms、#2 71.5 ms、#3 70.6 ms
（沒有紀錄）
```

探針結束 2 秒後再以 `pgrep -x iTerm2` 確認，iTerm2 仍未執行。

### `probe_cwd`（節錄測試視窗相關的 tty）

```
ps：628 支程序，前景 33 支（26 個 tty），耗時 83.7 ms
86436  ttys017  claude  claude                                       /Users/admin/Documents/cue-desk  =
86794  ttys017  other   /Users/admin/.codegraph/…/node --liftoff-only…  /Users/admin/Documents/cue-desk  =
86810  ttys017  other   /Users/admin/.codegraph/…/node -e \012const f…  /private/var/folders/…/T         =
86599  ttys018  other   vim                                          /Users/admin/Documents/cue-desk  =
89171  ttys024  claude  npm exec @anthropic-ai/claude-code           /Users/admin/Documents/cue-desk  =
90069  ttys024  claude  claude                                       /Users/admin/Documents/cue-desk  =
90107  ttys024  other   /Users/admin/.codegraph/…/node --liftoff-only…  /Users/admin/Documents/cue-desk  =
90112  ttys024  other   /Users/admin/.codegraph/…/node -e \012const f…  /private/var/folders/…/T         =
89323  ttys025  gemini  npm exec @google/gemini-cli                  /Users/admin/Documents/cue-desk  =
89863  ttys025  gemini  node <npx 快取>/_npx/…/node_modules/.bin/gemini  /Users/admin/Documents/cue-desk  =
89865  ttys025  gemini  …/node --max-old-space-size=8192 <npx 快取>/…/.bin/gemini  /Users/admin/Documents/cue-desk  =

libproc：33 支 pid，各輪耗時 98 µs / 42 µs / 40 µs（平均每支 1 µs）
lsof   ：33 支 pid 一次執行，各輪耗時 27.4 ms / 23.5 ms / 25.3 ms
```

（分割窗格 `ttys001`／`014`／`015`、最小化視窗 `ttys019` 皆為 `-zsh`，cwd 與 `cd` 的目標一致，兩種方式結果相同。）

前景程序群組明細（`ps -o pid,ppid,pgid,tpgid`）：

```
ttys017（原生 claude）
86436  ppid 84607  pgid 86436  tpgid 86436  claude                    ← 群組領頭
86794  ppid 86436  pgid 86436  tpgid 86436  …/codegraph/…/node        ← claude 的 MCP 子程序
86810  ppid 86794  pgid 86436  tpgid 86436  …/codegraph/…/node -e …

ttys024（npx 執行 npm 版 claude）
89171  ppid 88627  pgid 89171  tpgid 89171  npm exec @anthropic-ai/claude-code   ← 群組領頭
90069  ppid 89171  pgid 89171  tpgid 89171  claude                              ← npm 套件內附的原生執行檔
90107  ppid 90069  …                        …/codegraph/…/node

ttys025（npx 執行 gemini）
89323  ppid 88761  pgid 89323  tpgid 89323  npm exec @google/gemini-cli         ← 群組領頭
89863  ppid 89323  …                        node …/node_modules/.bin/gemini
89865  ppid 89863  …                        node --max-old-space-size=8192 …/.bin/gemini（gemini 重新啟動自己）
```

---

## 觀察與對正式實作的影響

### iTerm2 探索（`probe_discover`）

- **分割窗格各自一筆**，`unique ID` 與 `tty` 都不同，`tab#` 相同。符合「一個分頁內的多個窗格各自視為獨立工作階段」。
- **`win#`（視窗序號）會隨前後順序變動**：視窗 9782 在第一輪是 1，新視窗出現後變成 2。身分與比對一律用視窗 `id`，`win#` 只能用來顯示「視窗 N」。
- **視窗 `id`** 為整數，同一次執行期間穩定。iTerm2 重新啟動後原有視窗全數關閉，無法比較，依規格把舊節點標為 `exited` 即可。
- **`unique ID`** 為大寫 UUID。
- **`tty` 格式**：iTerm2 回傳 `/dev/ttys000`，`ps` 是 `ttys000`，比對前要去掉 `/dev/` 前綴。
- **`bounds`**：`{left, top, right, bottom}`，左上為原點。`left = 1586`／`2649` 表示位於主螢幕右側的第二螢幕。
- **最小化視窗**仍會列出，`bounds` 是正常（還原後）的位置（11905：`1470,643,2040,1080`），不會變成 0 或螢幕外座標。判斷所在螢幕時可直接使用；若要區分是否最小化，需另外讀 `miniaturized` 屬性。
- **`name`** 由 shell 或程式設定（`✳ Claude Code (claude)`、`◇  Ready (cue-desk) (node)`、`cd (vim)`），會頻繁變動，不能當身分，也不能用來判斷類型。
- **未執行時不會啟動**：iTerm2、Terminal.app 都已實測，`application id … is running` 的前置檢查有效；未執行時一次約 50–70 ms。
- **耗時隨工作階段數線性增加**：7 個約 690 ms、13 個約 1150 ms、16 個約 1400 ms（每個工作階段約 85 ms）。以 2 秒輪詢來看，**工作階段多時會吃掉大半間隔**，正式實作要：
  - 在背景執行緒（`spawn_blocking`）執行，一輪結束後才排下一輪，不重疊；
  - 腳本改為一次取回整批屬性（例如 `get {id, tty, name} of sessions of tabs of windows` 形式的大量取值），減少逐一 Apple Event 往返，並重新量測；
  - 若仍偏慢，調整輪詢間隔或提早採用 M6 事件訂閱。

### Terminal.app 探索

- **用 Cmd+T 開的分頁，在 AppleScript 中被回報為獨立的視窗**（新視窗 ID 11955，`tab#` 為 1），而不是同一視窗的第二個分頁。這是 macOS 原生分頁（window tabbing）的行為：每個分頁在 AppleScript 都是一個 window。
  - 同一分頁群組的「視窗」`bounds` 完全相同（11955 與 11920 都是 `1627,119,2207,540`），但不同群組也可能剛好同位置，不能據此可靠判斷。
  - 影響：Terminal.app 後端的「分頁序號」幾乎總是 1，「視窗 N · 分頁 M」的提示對 Terminal.app 要改為只顯示視窗；以「視窗 ID + tty」為身分仍然可行，且每個分頁的身分都是唯一的。
- `custom title` 預設為 `使用者@主機`，不是使用者設定的標題。
- 單一視窗加入分頁後，原視窗的 `bounds` 也會改變（11920 從 `1499,97,…` 變為 `1627,119,…`），代表分頁群組共享同一個框架。
- 耗時：2 個分頁約 310 ms、3 個約 400 ms。

### 前景程式與類型判斷（`probe_cwd`）

- **前景程序群組不只一支程序**：`claude` 會在同一群組啟動 MCP 伺服器等子程序（例如 `codegraph` 的 `node`），其中部分子程序的 cwd 甚至不同（`/private/var/folders/…/T`）。因此：
  - **「前景程式」應取群組領頭（`pid == tpgid`）**，cwd 也取群組領頭的 cwd；不能把群組內每支程序都當成前景程式。
  - 類型判斷先看群組領頭；**群組領頭是 `npm exec …`／`npx …`／`node` 等包裝程式時，再往群組內的子孫程序找** `claude`／`gemini`。
- **npm 版 `claude`（2.1.332）已改為套件內附的原生執行檔**（`@anthropic-ai/claude-code-darwin-arm64`，`bin` 為 `bin/claude.exe`），`ps` 顯示為 `claude`，**不再是 `node …/cli.js`**。透過 `npx` 執行時，群組領頭是 `npm exec @anthropic-ai/claude-code`。舊版 npm 套件的 `node …/claude-code/cli.js` 形式仍要支援（以單元測試的範例輸出涵蓋）。
- **`gemini`**：`node …/node_modules/.bin/gemini`，並會以 `node --max-old-space-size=8192 …/.bin/gemini` 重新啟動自己；透過 `npx` 時群組領頭是 `npm exec @google/gemini-cli`。判斷規則：`args` 中腳本路徑的檔名為 `gemini`，或 `npm exec` 的套件為 `@google/gemini-cli`。
- **`vim`**：`ps` 顯示 `vim`，類型為「其他（vim）」。
- **shell 形式**：iTerm2 的登入 shell 顯示為 `-zsh`（開頭帶 `-`），Claude 桌面 App 等則是 `/opt/homebrew/bin/zsh -l`。類型判斷時要去掉開頭的 `-` 再取執行檔名稱，並接受完整路徑。
- **`ps` 會看到其他程式的 tty**：第一輪 15 個前景 tty 中只有 7 個屬於 iTerm2，其餘的父程序是 Claude 桌面 App（`Claude Helper`）內建的終端機面板。**工作階段只能以 `discover()` 回報的 tty 為準**，`ps` 結果僅用來查前景程式，不得自行產生工作階段。

### cwd 取得方式

- **libproc 與 `lsof` 結果完全一致**（兩輪共 48 筆可取得的全部相同）。
- **耗時**：libproc 每支約 1 µs；`lsof` 批次一次 5–27 ms（隨 pid 數增加，另需啟動子程序）。**正式實作採用 libproc**（`proc_pidinfo` + `PROC_PIDVNODEPATHINFO`），不需 `lsof`。
- **非本使用者程序**：libproc 回 `EPERM`（errno 1），`lsof` 靜默無輸出。視為「無法取得 cwd」，該工作階段放入「未歸類」，不當成錯誤上報。
- **程序已結束**：前景程序在 `ps` 與取 cwd 之間結束時，libproc 回 `ESRCH`（errno 3）。屬正常競態，視為本輪無法取得、下一輪再試；並排除 Cue 自己的子程序（例如 `ps`）。
- `ps` 本身 580–630 支程序約 55–85 ms。

---

## 測試後還原

- 關閉測試建立的 iTerm2 視窗 11896、11905 與其中的程式；依使用者同意結束 iTerm2（原有視窗 9782 一併關閉，需使用者自行重新開啟 iTerm2）。
- 結束 Terminal.app（測試前未執行）。
- `npx` 使用 scratchpad 內的臨時 npm 快取（`npm_config_cache`），測完整個刪除；`~/.npm/_npx` 未變動。
- `gemini` 首次在 cue-desk 執行時寫入 `~/.gemini`：已刪除 `history/cue-desk`、`tmp/cue-desk`，從 `projects.json` 移除 cue-desk，`state.json` 還原為 `state.json.bak` 的內容。`oauth_creds.json` 被 `gemini` 刷新了登入權杖，為避免登入失效**未還原**。

# slice-8-1 安裝與發佈腳本

> 階段：slice-8 發佈（對應規格 M7）
> 上一個子切片：slice-7-2 iTerm2 Socket 引擎｜下一個子切片：slice-8-2 文件與首次發佈

## 目標

提供一行指令安裝（重跑即更新）的 `install.sh`、`uninstall.sh`，以及推送 `v*` tag 時自動打包 universal 版本並上傳 GitHub Release 的 workflow。

## 必讀

- `docs/PROMPT.md`「六、安裝、資安與發佈」第 2～4 點

## 完成條件

- `install.sh`、`uninstall.sh` 通過 `bash -n` 與 `shellcheck`。
- workflow 語法正確，推送 tag 後產出 `Baton-macos-universal.zip` 與 `.sha256`（實際執行在 8-2-H1）。

---

## Commits

### 👤 8-1-H1 提供 GitHub OWNER／REPO 與最終 Bundle ID
- 請告訴 Claude：
  1. GitHub 的 `<OWNER>/<REPO>`（例如 `yourname/baton`），repo 是否已建立且為 public（一行安裝需要公開下載）。
  2. 最終 Bundle identifier（目前暫定 `dev.baton.desktop`；**發佈後再改會讓使用者的自動化授權與設定失效**）。
- Claude 會把這兩項記錄到 `docs/design/002-release.md`，並在 8-1-01 使用。

### 8-1-01 feat: install.sh 一行安裝
- **UI**：否
- **預計檔案**（3）：`install.sh`、`src-tauri/tauri.conf.json`、`docs/design/002-release.md`
- **實作重點**：
  - `tauri.conf.json`：套用最終 `identifier`；明確設定 `bundle.macOS.minimumSystemVersion`（依 001 調查的 Tauri v2 最低需求），install.sh 使用同一個值。
  - `install.sh` 規格：
    - 全部邏輯包在 `main()` 函式內，**最後一行才呼叫 `main "$@"`**（避免下載不完整時執行到一半）；`set -euo pipefail`。
    - 檢查系統為 macOS（`uname -s` = `Darwin`）且 `sw_vers -productVersion` ≥ 最低版本，否則中止並說明。
    - 預設下載 `https://github.com/<OWNER>/<REPO>/releases/latest/download/Baton-macos-universal.zip` 與其 `.sha256`；環境變數 `BATON_VERSION=vX.Y.Z` 時改為 `releases/download/<版本>/…`（驗證格式 `^v[0-9]+\.[0-9]+\.[0-9]+`）。
    - 下載到 `mktemp -d` 暫存目錄（`curl -fsSL`），以 `shasum -a 256 -c` 驗證，不符即中止；以 `trap` 在結束時清除暫存目錄。
    - 若 Baton 正在執行，先 `osascript -e 'quit app "Baton"'` 並等待結束（最多數秒）。
    - 以 `ditto -x -k` 解壓，安裝到 `/Applications`；沒有寫入權限時改裝到 `~/Applications`（必要時建立）；**不使用 `sudo`**。覆蓋前先移除舊的 `Baton.app`。
    - 執行 `xattr -dr com.apple.quarantine` 與 `codesign --force --deep --sign -`，讓 App 可直接開啟。
    - 偵測 iTerm2（`/Applications/iTerm.app` 或 `mdfind "kMDItemCFBundleIdentifier == 'com.googlecode.iterm2'"`）、`claude`、`gemini`（`command -v`）；未安裝的只**印出**官方安裝方式提示，**不自動安裝任何東西**。
    - 最後 `open` 啟動 App，並印出 README「Hook 設定」段落網址（`https://github.com/<OWNER>/<REPO>#hook-設定`）；**不自動修改使用者的設定檔**。
    - 同一支腳本重跑即為更新。
  - `002-release.md`：記錄 OWNER／REPO、Bundle ID、最低系統版本。
- **驗證**：`bash -n install.sh`、`shellcheck install.sh`；以本機 `npm run tauri build` 產物手動打包 zip＋sha256，設定暫時環境變數（例如 `BATON_DOWNLOAD_BASE=file://…`，僅供測試、README 不記載）跑一次完整流程。

### 8-1-02 feat: uninstall.sh
- **UI**：否
- **預計檔案**（1）：`uninstall.sh`
- **實作重點**：
  - 同樣 `main()` 包覆、最後一行呼叫、`set -euo pipefail`。
  - 結束 App（`osascript -e 'quit app "Baton"'`），移除 `/Applications/Baton.app` 與 `~/Applications/Baton.app`（存在者）。
  - 詢問後才刪除 `~/.baton`（**預設保留**，`read -r -p "…[y/N]"`，非互動環境一律保留）。
  - 提醒使用者自行從 `~/.claude/settings.json` 與 `~/.gemini/settings.json` 移除 Hook 設定。
  - 不移除任何終端機工作階段。
- **驗證**：`bash -n`、`shellcheck`；本機執行（選 N 保留 `~/.baton`）後 App 已移除。

### 8-1-03 ci: GitHub Actions 發佈流程
- **UI**：否
- **預計檔案**（2）：`.github/workflows/release.yml`、`.github/workflows/ci.yml`
- **實作重點**：
  - `release.yml`：`on: push: tags: ['v*']`；`runs-on: macos-latest`；`permissions: contents: write`。
    - 安裝 Node（LTS，`npm ci`）、Rust stable 並加入 `aarch64-apple-darwin`、`x86_64-apple-darwin` 兩個 target。
    - `npm run tauri build -- --target universal-apple-darwin`。
    - 以 `ditto -c -k --keepParent "…/Baton.app" Baton-macos-universal.zip` 壓縮；`shasum -a 256 Baton-macos-universal.zip > Baton-macos-universal.zip.sha256`（格式需能被 `shasum -a 256 -c` 驗證）。
    - 上傳到該版 GitHub Release（以 `gh release create`／`gh release upload`，使用 `GITHUB_TOKEN`；避免引入不必要的第三方 action）。
    - **不使用 Apple Developer ID 簽章與公證**。
  - `ci.yml`：push／PR 時在 macOS runner 跑 `npm ci` 與 `npm run verify`（不打包），確保主幹健康。
  - 所有第三方 action 以完整 commit SHA 固定版本。
- **驗證**：`actionlint`（若已安裝）或以 YAML 解析檢查；實際執行於 8-2-H1。

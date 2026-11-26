# slice-8-2 文件與首次發佈

> 階段：slice-8 發佈（對應規格 M7）
> 上一個子切片：slice-8-1 安裝與發佈腳本｜下一個子切片：無（🚀 v0.1.0）

## 目標

完成公開發佈所需的 README（含第三方授權清單）、LICENSE 與開發指南，推送第一個版本 tag 並在乾淨帳號驗收一行安裝與更新。

## 必讀

- `docs/PROMPT.md`「六、…」第 5 點（README 必須說明）
- `docs/PROMPT.md`「七、…」第 4 點（全部完成後的說明）
- `docs/design/002-release.md`

## 完成條件（M7 驗收）

- 在乾淨的 macOS 使用者帳號以一行指令完成安裝並能啟動；重跑同一指令可完成更新。
- README 的所有連結與一行安裝指令中的 `<OWNER>/<REPO>` 都已替換為實際值。

---

## Commits

### 8-2-01 docs: README 完整說明與第三方授權清單
- **UI**：否
- **預計檔案**（3）：`README.md`、`LICENSE`、`THIRD_PARTY_LICENSES.md`
- **實作重點**：
  - `LICENSE`：MIT 全文（著作權人依 git 使用者名稱或使用者指定）。
  - `README.md`（保留 5-2-04 的「Hook 設定」段落與其錨點）：
    - 產品介紹（不內嵌終端機、適用任何本機資料夾、0 額外 AI Token）。
    - 一行安裝：`curl -fsSL https://raw.githubusercontent.com/<OWNER>/<REPO>/main/install.sh | bash`；指定版本 `CUE_VERSION=vX.Y.Z`；重跑即更新；解除安裝方式。
    - 首次使用會出現「Cue 想要控制 iTerm2」的授權。
    - 因為沒有 Developer ID 簽章，每次更新後 macOS 可能再次詢問這項授權。
    - 若從瀏覽器手動下載 zip 開啟會被 Gatekeeper 阻擋，請使用一行指令安裝。
    - 選配：於 iTerm2 勾選 `Settings › General › Magic › Enable Python API` 可解鎖「從混裝視窗精準抽分頁／一鍵按工作區分家」（無須安裝 Python 或開啟輔助使用權限；首次連線 iTerm2 可能詢問一次）。
    - 內建終端機（Terminal.app）後備模式下不支援：分割窗格、新分頁、拖檔派工。
    - 資安說明：零網路 Port、Unix Domain Socket、Cookie／Key 只存在記憶體、不跳過 AI 權限確認。
    - iTerm2 協定來源與授權說明（依 001 決定）。
  - `THIRD_PARTY_LICENSES.md`：主要 Rust crate 與 npm 套件（名稱、版本、授權）、7 套 OFL 字型（引用 `src/assets/fonts/LICENSES.md`）；可用 `cargo metadata`／`npm ls --json` 輔助產生後人工整理，確認**沒有 GPL／AGPL**。
- **驗證**：README 中的指令逐一複製執行可用；`grep -n '<OWNER>' README.md install.sh` 無結果；授權清單無 GPL／AGPL。

### 8-2-02 docs: 開發、打包與發佈指南
- **UI**：否
- **預計檔案**（2）：`docs/DEVELOPMENT.md`、`README.md`
- **實作重點**：
  - 開發預覽：`npm install` → `npm run tauri dev`；`npm run verify` 的內容。
  - 本機打包：`npm run tauri build`（以及 `-- --target universal-apple-darwin` 需先 `rustup target add`）。
  - 發佈新版本：更新 `package.json`、`src-tauri/Cargo.toml`、`tauri.conf.json` 版本號 → commit → `git tag vX.Y.Z` → `git push origin vX.Y.Z`，等待 workflow 完成。
  - Hook 設定範例的位置（`docs/hooks/`、README 段落），強調只提供範例、不自動修改使用者設定。
  - 專案結構與切片開發流程（指向 `CLAUDE.md`、`docs/STATUS.md`）。
  - README 加一行連結到 `docs/DEVELOPMENT.md`。
- **驗證**：依文件從乾淨 clone 走一次 `npm install` 與 `npm run tauri dev`。

### 👤 8-2-H1 推送 v0.1.0 tag 並在乾淨帳號驗收
- 合併到 `main` 後，更新三處版本號為 `0.1.0`，推送 tag：`git tag v0.1.0 && git push origin v0.1.0`（Claude 不會自行 push）。
- 確認 GitHub Actions 成功、Release 有 `Cue-macos-universal.zip` 與 `.sha256`。
- 在 macOS 新增一個乾淨的使用者帳號，執行 README 的一行安裝指令，確認可啟動；再執行一次確認更新流程正常。
- 回報結果給 Claude，記錄在本文件末尾「驗收紀錄」。完成即達成 🚀 v0.1.0 里程碑。

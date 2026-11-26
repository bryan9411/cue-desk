# slice-1-1 專案骨架

> 階段：slice-1 技術驗證與專案骨架（對應規格第七節第 1、2 點）
> 上一個子切片：無｜下一個子切片：slice-1-2 技術探針

## 目標

建立可以 `npm run tauri dev` 啟動的空白 Baton 視窗，並把安全基線（CSP、最小權限 capabilities、自動化權限宣告）一次設好，之後所有切片都在這個基線上開發。

## 必讀

- `docs/PROMPT.md`「六、安裝、資安與發佈」第 1 點（資安規範）——1-1-03 需要。

## 完成條件

- `npm run tauri dev` 開出一個標題為 Baton、約 `320 × 850` 的視窗。
- `npm run verify` 全部通過（cargo check、clippy、cargo test、前端 build、Vitest）。
- CSP、capabilities、`Info.plist`、entitlements 已依規格設定。

---

## Commits

### 1-1-01 chore: 初始化 Tauri v2 + React + TypeScript 專案
- **UI**：否
- **預計檔案**（8）：`package.json`、`vite.config.ts`、`tsconfig.json`、`index.html`、`src/main.tsx`、`src/App.tsx`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`（不計入：`npm create tauri-app` 產生且未修改的 `src-tauri/src/main.rs`、`src-tauri/build.rs`、`src-tauri/icons/*`、`.gitignore`、lockfile）
- **實作重點**：
  - **先檢查環境**：`rustc --version`、`cargo --version`、`node --version`、`xcode-select -p`。缺少任何一項就**停下**，提示安裝指令（Rust：`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`；Node：建議 LTS；Xcode CLT：`xcode-select --install`），不要繼續。
  - 以 `npm create tauri-app@latest`（React + TypeScript 範本、npm）在專案根目錄建立；若工具不允許非空目錄，先產生到暫存目錄再搬移，保留既有的 `CLAUDE.md`、`docs/`、`README.md`。
  - `tauri.conf.json`：`productName: "Baton"`、`identifier: "dev.baton.desktop"`（暫定，8-1-H1 定案）、主視窗 label `main`、`width: 320`、`height: 850`、`minWidth: 280`。
  - Rust lib crate 名稱 `baton_lib`（後續 `examples/` 探針會引用）。
  - 移除範本的 greet 範例、logo 與 CSS，`App.tsx` 只顯示純文字「Baton」。
  - Rust edition 2021，加入 `tokio`（`rt-multi-thread`、`macros`、`time`、`sync`、`net`、`io-util`）、`serde`（derive）、`serde_json`。
- **驗證**：`npm run tauri dev` 出現視窗；`cargo check --manifest-path src-tauri/Cargo.toml`、`npm run build` 通過。

### 1-1-02 chore: 加入 Tailwind CSS、Zustand 與 Vitest
- **UI**：否
- **預計檔案**（6）：`package.json`、`vite.config.ts`、`src/index.css`、`src/main.tsx`、`src/store/useBaton.ts`、`src/__tests__/smoke.test.ts`
- **實作重點**：
  - Tailwind CSS v4（`@tailwindcss/vite`），`src/index.css` 以 `@import "tailwindcss";` 載入。
  - **主題變數橋接**：在 `index.css` 以 `@theme inline` 把 Tailwind 的色彩／字型／圓角對應到主題 CSS Variables，例如 `--color-base: var(--bg-base)`、`--color-surface: var(--bg-surface)`、`--color-accent: var(--accent)`、`--font-heading: var(--font-heading)`、`--radius-sm: var(--radius-sm)`……（涵蓋 PROMPT 第五節第 2 點的所有變數）。之後元件只用這些 utility class 或 `var(--…)`，不寫硬編碼值。
  - 在 `:root` 先放「暖紙雜誌 `paper`」的暫用值（slice-4-1 由主題引擎接手）：`--bg-base`/`--bg-sidebar: #F5F1E8`、`--bg-surface: #EBE4D4`、`--bg-selected: #E2D6BE`、`--border-subtle: #D3C7B0`、`--border-strong: #2B2620`、`--text-primary: #2B2620`、`--text-muted: #685D4E`、`--accent: #D97757`、`--on-accent: #2B2620`、`--focus-ring: #D97757`、`--status-running: #3A6BA5`、`--status-waiting: #B4741A`、`--status-done: #3D7A44`、`--status-idle: #8A7F70`、`--status-exited: #B3A893`、`--font-heading: ui-serif, "New York", "Songti TC", serif`、`--font-body: -apple-system, "SF Pro Text", "PingFang TC", sans-serif`、`--font-mono: ui-monospace, "SF Mono", Menlo, monospace`、`--heading-weight: 600`、`--title-size: 20px`、`--radius-sm: 6px`、`--radius-lg: 12px`、`--border-width: 1px`、`--border-style: solid`、`--shadow-control: none`、`--shadow-panel: none`、`--texture: none`、`--status-shape: dot`。
  - Zustand：`src/store/useBaton.ts` 建立空的 store 骨架（`sessions`、`workspaces` 先為空陣列）。
  - Vitest：設定在 `vite.config.ts` 的 `test` 欄位（`environment: "jsdom"`），`package.json` 加 `"test": "vitest run"`；`smoke.test.ts` 驗證 store 可建立。
- **驗證**：`npm test`、`npm run build` 通過；dev 視窗背景為暖紙色。

### 1-1-03 chore: 設定 CSP、最小權限 capabilities 與自動化權限宣告
- **UI**：否
- **預計檔案**（7）：`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`、`src-tauri/Info.plist`、`src-tauri/Entitlements.plist`、`src-tauri/Cargo.toml`、`src-tauri/src/lib.rs`、`package.json`
- **實作重點**：
  - **CSP**（`app.security.csp`）原樣使用：`default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src ipc: http://ipc.localhost; object-src 'none'; frame-ancestors 'none'`。若日後有依賴必須動態插入 `<style>`，才改 `style-src 'self' 'unsafe-inline'` 並設 `"dangerousDisableAssetCspModification": ["style-src"]`，且在程式碼註明原因（目前不需要；React 的 `style` 屬性走 CSSOM，不受 CSP 限制）。
  - **Capabilities**：單一 `default.json`，`windows: ["main", "prefs"]`（只涵蓋這兩個 label）。權限只開：`core:default`、`core:window:allow-start-dragging`、`core:window:allow-set-theme`、`core:window:allow-set-size`、`core:window:allow-set-position`、`core:window:allow-set-always-on-top`、`core:webview:allow-create-webview-window`、`dialog:allow-open`、`dialog:allow-ask`、`opener:allow-reveal-item-in-dir`、`opener:allow-open-url`（scope 限 `http://*`、`https://*`）。notification 權限於 5-2-02 加入。自訂指令權限由 `invoke_handler` 註冊的 commands 決定，之後每新增 command 不需改 capability（Tauri v2 app commands 預設允許）。
  - 加入外掛：`tauri-plugin-dialog`、`tauri-plugin-opener`（Rust 與 npm 兩端），在 `lib.rs` 註冊。
  - `Info.plist`：`NSAppleEventsUsageDescription` =「Baton 需要控制 iTerm2／終端機，以開啟與切換你的工作階段」。
  - `Entitlements.plist`：`com.apple.security.automation.apple-events` = true；`tauri.conf.json` 的 `bundle.macOS.entitlements` 指向它。
  - **不得**啟用任何 HTTP 伺服器類外掛（例如 `tauri-plugin-localhost`）。
- **驗證**：`npm run tauri dev` 正常啟動、DevTools Console 無 CSP 錯誤；`npm run tauri build` 後以 `plutil -p` 確認 `Baton.app/Contents/Info.plist` 含 `NSAppleEventsUsageDescription`。

### 1-1-04 chore: 加入 verify 檢查指令
- **UI**：否
- **預計檔案**（2）：`scripts/verify.sh`、`package.json`
- **實作重點**：
  - `scripts/verify.sh`（`set -euo pipefail`）依序執行：`cargo check`、`cargo clippy -- -D warnings`、`cargo test`（皆帶 `--manifest-path src-tauri/Cargo.toml`）、`npm run build`、`npm test`；若 `package.json` 有 `lint:style` 腳本則一併執行（4-1-05 之後才有）。
  - `package.json` 加 `"verify": "bash scripts/verify.sh"`。
- **驗證**：`npm run verify` 全部通過。

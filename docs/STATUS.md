# Cue 開發進度

## 使用方式

- 說「**請繼續實作**」，Claude 會用 `grep -n -m1 -E '^  - \[ \]' docs/STATUS.md` 找到下一個未勾選項目，只讀對應的切片文件，實作一個 Commit 後停下等你確認。
- 👤 開頭的是**人工項目**：Claude 不會實作，會告訴你需要做什麼。完成後告訴 Claude，它會記錄結果並勾選。
- 建議每完成一個 Commit 就開新的 session，再說「請繼續實作」，避免對話累積消耗 token。
- Commit 編號：`X-Y-NN`（切片 X、子切片 Y、第 NN 個）；超過檔案上限時拆成 `X-Y-NNa`、`X-Y-NNb`。

## 階段總表

| 切片 | 對應階段 | 說明 | 完成後可否上線 |
|---|---|---|---|
| slice-1 | 第七節第 1、2 點（前置） | 專案骨架、技術探針、技術決策紀錄 | 否 |
| slice-2 | M1 探索與認領 | 終端機後端、程序解析、cwd 自動歸屬、基礎側邊欄 | 否（可自用：唯讀檢視與聚焦） |
| slice-3 | M2 建立、管理、防呆與拖檔派工 | 建立／補切／命名／關閉、Git 分支、拖放 | 否（可自用） |
| slice-4 | M3 外觀、面板與效率 | 主題引擎、16 套主題、偏好設定視窗、雙型態外殼、`Cmd+P` | 否（可自用） |
| slice-5 | M4 Hook 與通知 | Unix socket Hook、`cue-notify`、提示音、通知、Dock 徽章 | 否（功能已完整可自用，尚無發佈管道） |
| slice-6 | M5 雜誌編輯室角色 | 5 種狀態 SVG 角色與動畫 | 否 |
| slice-7 | M6 雙螢幕與 iTerm2 Socket 精準分頁 | 螢幕座標、並排終端、送到螢幕 2、Socket 引擎 | 否 |
| slice-8 | M7 發佈 | `install.sh`、`uninstall.sh`、Release workflow、README | **是**（🚀 v0.1.0） |

---

## slice-1 技術驗證與專案骨架

- [ ] **slice-1-1 專案骨架** → `docs/slices/slice-1-1.md`
  - [ ] 1-1-01 chore: 初始化 Tauri v2 + React + TypeScript 專案
  - [ ] 1-1-02 chore: 加入 Tailwind CSS、Zustand 與 Vitest
  - [ ] 1-1-03 chore: 設定 CSP、最小權限 capabilities 與自動化權限宣告
  - [ ] 1-1-04 chore: 加入 verify 檢查指令
- [ ] **slice-1-2 技術探針** → `docs/slices/slice-1-2.md`
  - [ ] 1-2-01 feat: osascript 執行器與分隔字元解析
  - [ ] 1-2-02 spike: iTerm2 與 Terminal.app 探索腳本探針
  - [ ] 1-2-03 spike: 前景程式工作目錄取得方式探針
  - [ ] 1-2-04 docs: 外部依賴與協定調查筆記
  - [ ] 👤 1-2-H1 在 iTerm2／Terminal.app 準備情境並回報探針輸出
- [ ] **slice-1-3 技術決策** → `docs/slices/slice-1-3.md`
  - [ ] 👤 1-3-H1 決定 api.proto 與通知外掛的處理方式
  - [ ] 1-3-01 docs: 加入技術決策紀錄並依結論更新實作說明

## slice-2 探索與認領（M1）

- [ ] **slice-2-1 領域模型與程序解析** → `docs/slices/slice-2-1.md`
  - [ ] 2-1-01 feat: 定義工作階段領域型別與 TerminalBackend trait
  - [ ] 2-1-02 feat: 解析 ps 輸出並判斷前景程式與類型
  - [ ] 2-1-03 feat: 取得前景程式的工作目錄
  - [ ] 2-1-04 feat: 以 realpath 最長前綴歸屬工作區
- [ ] **slice-2-2 終端機後端** → `docs/slices/slice-2-2.md`
  - [ ] 2-2-01 feat: 偵測已安裝的終端機與執行狀態
  - [ ] 2-2-02 feat: 實作 ITerm2Backend 的探索與聚焦
  - [ ] 2-2-03 feat: 實作 TerminalAppBackend 的探索與聚焦
- [ ] **slice-2-3 探索迴圈與設定** → `docs/slices/slice-2-3.md`
  - [ ] 2-3-01 feat: 設定檔模型與原子寫入
  - [ ] 2-3-02 feat: 合併探索結果並管理節點生命週期
  - [ ] 2-3-03 feat: 每 2 秒探索迴圈與 IPC 指令、事件
- [ ] **slice-2-4 基礎側邊欄** → `docs/slices/slice-2-4.md`
  - [ ] 2-4-01 feat: 前端型別、IPC 包裝與 Zustand store
  - [ ] 2-4-02 feat: 樹狀側邊欄與點擊聚焦
  - [ ] 👤 2-4-H1 M1 實機驗收

## slice-3 建立、管理、防呆與拖檔派工（M2）

- [ ] **slice-3-1 安全指令層與建立工作階段** → `docs/slices/slice-3-1.md`
  - [ ] 3-1-01 feat: shell 單引號跳脫與 AI 指令列舉
  - [ ] 3-1-02 feat: 開啟方式與專屬視窗判定
  - [ ] 3-1-03 feat: iTerm2 建立工作階段（四種開啟方式）
  - [ ] 3-1-04 feat: Terminal.app 建立工作階段與 create 指令
- [ ] **slice-3-2 工作階段管理** → `docs/slices/slice-3-2.md`
  - [ ] 3-2-01 feat: 就地補開同工作區 Zsh
  - [ ] 3-2-02 feat: 重新命名與關閉工作階段
  - [ ] 3-2-03 feat: 重新開啟、移除與未歸類操作
- [ ] **slice-3-3 選單與建立 Modal** → `docs/slices/slice-3-3.md`
  - [ ] 3-3-01 feat: 右鍵選單與節點操作
  - [ ] 3-3-02 feat: 建立工作階段 Modal 與多 AI 防呆
- [ ] **slice-3-4 Git 分支監聽** → `docs/slices/slice-3-4.md`
  - [ ] 3-4-01 feat: 解析 Git 目錄與 HEAD
  - [ ] 3-4-02 feat: 以 notify 監聽分支變化
  - [ ] 3-4-03 feat: 側邊欄分支徽章
- [ ] **slice-3-5 拖放與派工** → `docs/slices/slice-3-5.md`
  - [ ] 3-5-01 feat: 拖檔派工規則與不送 Enter 的文字輸入
  - [ ] 3-5-02 feat: 拖曳資料夾加入工作區與拖檔到節點
  - [ ] 👤 3-5-H1 M2 實機驗收

## slice-4 外觀、面板與效率（M3）

- [ ] **slice-4-1 主題引擎** → `docs/slices/slice-4-1.md`
  - [ ] 4-1-01 feat: 主題型別與 8 套風格主題
  - [ ] 4-1-02 feat: 色彩工具、推導規則與 8 套經典配色
  - [ ] 👤 4-1-H1 同意下載 OFL 字型檔
  - [ ] 4-1-03 feat: 打包 OFL 字型
  - [ ] 4-1-04 feat: 主題服務：套用、預覽與提交
  - [ ] 4-1-05 test: 禁止硬編碼樣式的自動檢查
- [ ] **slice-4-2 主視窗外殼** → `docs/slices/slice-4-2.md`
  - [ ] 4-2-01 feat: Overlay 標題列、標誌與細線圖示集
  - [ ] 4-2-02 feat: 狀態燈支援 geometric 形狀
  - [ ] 4-2-03 feat: 窄版與寬版雙型態與視窗狀態持久化
  - [ ] 4-2-04 feat: 底部控制列、視窗置頂與音效開關
  - [ ] 4-2-05 feat: 原生選單（App、Edit、顯示 › 主題）
  - [ ] 4-2-06 feat: 毛玻璃與背景透明度
  - [ ] 4-2-07 feat: 自動化權限被拒橫幅
- [ ] **slice-4-3 偏好設定視窗** → `docs/slices/slice-4-3.md`
  - [ ] 4-3-01 feat: 偏好設定獨立視窗骨架
  - [ ] 4-3-02 feat: 一般、終端機、通知、進階分頁
  - [ ] 4-3-03 feat: 外觀分頁的主題縮圖與即時預覽
  - [ ] 4-3-04 feat: 跟隨系統深淺色
  - [ ] 4-3-05 feat: 背景透明度滑桿
- [ ] **slice-4-4 .itermcolors 匯入** → `docs/slices/slice-4-4.md`
  - [ ] 4-4-01 feat: 解析 .itermcolors 並轉換色彩空間
  - [ ] 4-4-02 feat: 我的主題存取與管理
- [ ] **slice-4-5 指令面板與首次啟動** → `docs/slices/slice-4-5.md`
  - [ ] 4-5-01 feat: 模糊搜尋與指令註冊表
  - [ ] 4-5-02 feat: Cmd+P 指令面板
  - [ ] 4-5-03 feat: 首次啟動外觀卡片
  - [ ] 👤 4-5-H1 M3 實機驗收

## slice-5 Hook 與通知（M4）

- [ ] **slice-5-1 Hook 通道** → `docs/slices/slice-5-1.md`
  - [ ] 5-1-01 feat: Unix Domain Socket Hook 伺服器
  - [ ] 5-1-02 feat: cue-notify 輔助腳本
  - [ ] 5-1-03 feat: 前端狀態燈號連動
- [ ] **slice-5-2 提示與通知** → `docs/slices/slice-5-2.md`
  - [ ] 5-2-01 feat: Web Audio 雙頻提示音
  - [ ] 5-2-02 feat: macOS 原生通知與點擊聚焦
  - [ ] 5-2-03 feat: Dock 待決數量徽章
  - [ ] 5-2-04 docs: Claude Code 與 Gemini CLI Hook 設定範例
  - [ ] 👤 5-2-H1 M4 實機驗收

## slice-6 雜誌編輯室角色（M5）

- [ ] **slice-6-1 角色造型** → `docs/slices/slice-6-1.md`
  - [ ] 6-1-01 feat: 角色 SVG 基礎與 idle 靜態造型
  - [ ] 👤 6-1-H1 確認角色風格
- [ ] **slice-6-2 狀態動畫** → `docs/slices/slice-6-2.md`
  - [ ] 6-2-01 feat: running 與 waiting 姿勢與動畫
  - [ ] 6-2-02 feat: done、idle 動畫與 exited 空座位
  - [ ] 6-2-03 feat: 狀態過場、隱藏暫停與減少動態
  - [ ] 👤 6-2-H1 M5 實機驗收

## slice-7 雙螢幕與 iTerm2 Socket 精準分頁（M6）

- [ ] **slice-7-1 螢幕座標與整窗調度** → `docs/slices/slice-7-1.md`
  - [ ] 7-1-01 feat: 螢幕資訊與座標換算
  - [ ] 7-1-02 feat: 並排終端
  - [ ] 7-1-03 feat: 送到螢幕 2 與收回螢幕 1
  - [ ] 7-1-04 feat: 工作區收回與分頁整理（整窗模式）
  - [ ] 7-1-05 feat: 雙螢幕與整理操作的前端入口
- [ ] **slice-7-2 iTerm2 Socket 引擎** → `docs/slices/slice-7-2.md`
  - [ ] 7-2-01 feat: iTerm2 API 最小協定子集
  - [ ] 7-2-02 feat: 取得 Cookie／Key 並建立 Socket 連線
  - [ ] 7-2-03 feat: 透過 Socket 列出工作階段與事件訂閱
  - [ ] 7-2-04 feat: 精準分頁操作
  - [ ] 7-2-05 feat: Socket 引擎設定與降級提示
  - [ ] 👤 7-2-H1 M6 實機驗收（雙螢幕）

## slice-8 發佈（M7）

- [ ] **slice-8-1 安裝與發佈腳本** → `docs/slices/slice-8-1.md`
  - [ ] 👤 8-1-H1 確認 GitHub repo 與最終 Bundle ID
  - [ ] 8-1-01 feat: install.sh 一行安裝
  - [ ] 8-1-02 feat: uninstall.sh
  - [ ] 8-1-03 ci: GitHub Actions 發佈流程
- [ ] **slice-8-2 文件與首次發佈** → `docs/slices/slice-8-2.md`
  - [ ] 8-2-01 docs: README 完整說明與第三方授權清單
  - [ ] 8-2-02 docs: 開發、打包與發佈指南
  - [ ] 👤 8-2-H1 推送 v0.1.0 tag 並在乾淨帳號驗收

### 🚀 里程碑：v0.1.0 公開發佈

- **完成 8-2-H1 即可上線**：GitHub Release 提供 `Cue-macos-universal.zip` 與 `.sha256`，README 的一行指令可安裝與更新。
- **包含**：M1～M7 全部功能——工作區與工作階段認領、建立／補切／命名／關閉、拖檔派工、Git 分支徽章、16 套內建主題與 `.itermcolors` 匯入、偏好設定視窗、`Cmd+P`、Hook 狀態燈號／提示音／通知／Dock 徽章、雜誌編輯室角色動畫、雙螢幕調度、iTerm2 Socket 精準分頁（選配）。
- **不包含**：Apple Developer ID 簽章與公證（每次更新 macOS 可能重新詢問自動化授權）、自動更新機制、Ghostty／WezTerm 等其他終端機後端、`cliSessionId`／`transcriptPath` 的實際應用、Git Worktree 管理 UI。

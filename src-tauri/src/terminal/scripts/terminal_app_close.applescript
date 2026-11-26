-- 關閉 Terminal.app 的指定分頁。
-- 不 activate；Terminal.app 未執行時直接回傳空字串，不啟動它。
--
-- argv：
--   1. 視窗的 window id（整數）
--   2. 分頁的 tty（/dev/ttysNNN）
--
-- 回傳：
--   "ok"     已送出關閉
--   "multi"  該視窗不只 1 個分頁，未關閉
--   ""       找不到視窗，或視窗的 tty 不符
--
-- 以 Cmd+T 開的原生分頁在 AppleScript 中是獨立的視窗，close 該視窗只會關閉這一個分頁。
-- 視窗回報多個分頁時代表不是原生分頁，close 會連同其他分頁一起關閉，因此拒絕。
-- 程式仍在執行時由 Terminal.app 自己決定是否跳出確認；close 以 ignoring application responses
-- 送出，不等確認對話框結束。實際是否關閉由下一輪探索判定。
on run argv
	set targetWinId to (item 1 of argv) as integer
	set targetTty to item 2 of argv

	if not (application id "com.apple.Terminal" is running) then return ""

	tell application id "com.apple.Terminal"
		try
			set w to window id targetWinId
			set tabTtys to tty of tabs of w
		on error number errNum
			-- 視窗已關閉：回傳找不到；其他錯誤（例如權限）照常拋出。
			if errNum is -1728 then return ""
			error number errNum
		end try

		if tabTtys does not contain targetTty then return ""
		if (count of tabTtys) is not 1 then return "multi"

		ignoring application responses
			close w
		end ignoring
	end tell

	return "ok"
end run

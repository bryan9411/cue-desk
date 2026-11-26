-- 在 iTerm2 開新視窗並送出指令。
-- 由使用者主動建立觸發，因此會 activate iTerm2（未執行時會啟動）。
--
-- argv：
--   1. 要送進新窗格的指令（已在呼叫端跳脫，腳本只原樣轉送）
--
-- 回傳新窗格的 unique ID。
on run argv
	set cmd to item 1 of argv

	tell application id "com.googlecode.iterm2"
		activate
		set w to (create window with default profile)
		set s to current session of w
		tell s to write text cmd
		return (unique ID of s) as text
	end tell
end run

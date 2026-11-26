-- 在 iTerm2 開新分頁並送出指令。
-- 由使用者主動建立觸發，因此會 activate iTerm2（未執行時會啟動）。
--
-- argv：
--   1. 要送進新窗格的指令（已在呼叫端跳脫，腳本只原樣轉送）
--   2. 目標視窗 ID；空字串表示最前視窗
--
-- 目標視窗已關閉時改用最前視窗；沒有任何視窗時改開新視窗。
-- 回傳新窗格的 unique ID。
on run argv
	set cmd to item 1 of argv
	set targetId to item 2 of argv

	set s to my newTabSession(targetId)
	tell application id "com.googlecode.iterm2"
		tell s to write text cmd
		return (unique ID of s) as text
	end tell
end run

-- 開新分頁並回傳其窗格；目標視窗會被帶到最前，使用者能看到新分頁。
on newTabSession(targetId)
	tell application id "com.googlecode.iterm2"
		activate
		set w to missing value
		if targetId is not "" then
			try
				set w to window id (targetId as integer)
				get id of w
			on error
				set w to missing value
			end try
		end if
		if w is missing value then
			try
				set w to current window
			on error
				set w to missing value
			end try
		end if

		if w is missing value then
			set w to (create window with default profile)
			return current session of w
		end if

		try
			if miniaturized of w then set miniaturized of w to false
		end try
		select w
		tell w to set t to (create tab with default profile)
		return current session of t
	end tell
end newTabSession

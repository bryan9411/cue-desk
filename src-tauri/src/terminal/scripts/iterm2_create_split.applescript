-- 在 iTerm2 開新分頁跑 AI，並在同一分頁分割出 zsh 窗格。
-- 由使用者主動建立觸發，因此會 activate iTerm2（未執行時會啟動）。
--
-- argv：
--   1. 要送進 AI 窗格的指令（已在呼叫端跳脫，腳本只原樣轉送）
--   2. 目標視窗 ID；空字串表示最前視窗
--   3. 要送進 zsh 窗格的指令（同樣已跳脫）
--   4. 分割方向："horizontal"（水平分隔線，zsh 在下）或 "vertical"（垂直分隔線，zsh 在右）
--
-- 目標視窗已關閉時改用最前視窗；沒有任何視窗時改開新視窗。
-- 回傳：AI 窗格 unique ID ␟ zsh 窗格 unique ID（以 ASCII 31 分隔）。
on run argv
	set aiCmd to item 1 of argv
	set targetId to item 2 of argv
	set zshCmd to item 3 of argv
	set direction to item 4 of argv

	if direction is not "horizontal" and direction is not "vertical" then
		error "未知的分割方向" number -1703
	end if

	set s to my newTabSession(targetId)
	tell application id "com.googlecode.iterm2"
		tell s to write text aiCmd
		-- iTerm2 的命名以分隔線方向為準：horizontally 是新窗格在下，vertically 是新窗格在右。
		if direction is "horizontal" then
			tell s to set z to (split horizontally with default profile)
		else
			tell s to set z to (split vertically with default profile)
		end if
		tell z to write text zshCmd
		return ((unique ID of s) as text) & (ASCII character 31) & ((unique ID of z) as text)
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

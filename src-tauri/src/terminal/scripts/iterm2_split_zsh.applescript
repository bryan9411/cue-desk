-- 在 iTerm2 的指定窗格旁分割出 zsh 窗格，並只對新窗格送出指令。
-- 由使用者主動操作觸發；目標窗格必定存在，因此 iTerm2 未執行時直接回傳空字串，不啟動它。
--
-- argv：
--   1. 目標窗格的 unique ID
--   2. 分割方向："vertical"（垂直分隔線，新窗格在右）或 "horizontal"（水平分隔線，新窗格在下）
--   3. 要送進新窗格的指令（已在呼叫端跳脫，腳本只原樣轉送）
--
-- 原窗格不送任何輸入：AI 對話照常進行。
-- 找到時回傳新窗格的 unique ID；找不到目標窗格時回傳空字串。
on run argv
	set targetId to item 1 of argv
	set direction to item 2 of argv
	set cmd to item 3 of argv

	if direction is not "horizontal" and direction is not "vertical" then
		error "未知的分割方向" number -1703
	end if
	if not (application id "com.googlecode.iterm2" is running) then return ""

	tell application id "com.googlecode.iterm2"
		set winIds to id of windows
	end tell

	repeat with winId in winIds
		set winId to contents of winId
		try
			tell application id "com.googlecode.iterm2"
				set tabIds to unique ID of sessions of tabs of window id winId
			end tell
			repeat with ti from 1 to count of tabIds
				set sessIds to item ti of tabIds
				repeat with si from 1 to count of sessIds
					if ((item si of sessIds) as text) is targetId then
						return my splitAt(winId, ti, si, direction, cmd)
					end if
				end repeat
			end repeat
		on error number errNum
			-- 視窗在走訪期間被關閉：略過；其他錯誤（例如權限）照常拋出。
			if errNum is not -1728 then error number errNum
		end try
	end repeat

	return ""
end run

on splitAt(winId, ti, si, direction, cmd)
	tell application id "com.googlecode.iterm2"
		activate
		set w to window id winId
		try
			if miniaturized of w then set miniaturized of w to false
		end try
		select w
		select tab ti of w
		set s to session si of tab ti of w
		-- iTerm2 的命名以分隔線方向為準：vertically 是新窗格在右，horizontally 是新窗格在下。
		if direction is "vertical" then
			tell s to set z to (split vertically with default profile)
		else
			tell s to set z to (split horizontally with default profile)
		end if
		tell z to write text cmd
		return (unique ID of z) as text
	end tell
end splitAt

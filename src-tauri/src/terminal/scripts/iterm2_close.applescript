-- 關閉 iTerm2 的指定窗格（session）。
-- 不 activate；iTerm2 未執行時直接回傳空字串，不啟動它。
--
-- argv：
--   1. 窗格的 unique ID
--
-- 找到時回傳 "ok"；找不到時回傳空字串。
-- 程式仍在執行時由 iTerm2 自己決定是否跳出確認；close 以 ignoring application responses
-- 送出，不等確認對話框結束，否則 osascript 會卡到逾時。實際是否關閉由下一輪探索判定。
on run argv
	set targetId to item 1 of argv

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
						tell application id "com.googlecode.iterm2"
							set s to session si of tab ti of window id winId
							ignoring application responses
								close s
							end ignoring
						end tell
						return "ok"
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

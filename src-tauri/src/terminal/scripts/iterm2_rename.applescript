-- 設定 iTerm2 指定窗格（session）的名稱，讓分頁標題顯示使用者的標籤。
-- 不 activate、不切換焦點；iTerm2 未執行時直接回傳空字串，不啟動它。
--
-- argv：
--   1. 窗格的 unique ID
--   2. 新名稱（已在呼叫端驗證，腳本只原樣設定）
--
-- 找到時回傳 "ok"；找不到時回傳空字串。
-- 使用者的 shell 若會覆寫標題，之後以 shell 設定的為準。
on run argv
	set targetId to item 1 of argv
	set newName to item 2 of argv

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
							set name of session si of tab ti of window id winId to newName
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

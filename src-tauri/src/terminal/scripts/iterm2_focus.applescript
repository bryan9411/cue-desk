-- 聚焦 iTerm2 的指定窗格（session）。
-- 由使用者主動操作觸發，因此會 activate iTerm2（未執行時會啟動）。
--
-- argv：
--   1. 窗格的 unique ID
--
-- 找到時回傳 "ok"；找不到時回傳空字串。
-- 以批次取值找出窗格所在的視窗、分頁與序號，再依序 select 視窗 → 分頁 → 窗格。
on run argv
	set targetId to item 1 of argv

	tell application id "com.googlecode.iterm2"
		activate
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
							set w to window id winId
							try
								if miniaturized of w then set miniaturized of w to false
							end try
							select w
							select tab ti of w
							select session si of tab ti of w
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

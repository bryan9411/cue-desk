-- 聚焦 Terminal.app 的指定分頁。
-- 由使用者主動操作觸發，因此會 activate Terminal.app（未執行時會啟動）。
--
-- argv：
--   1. 視窗的 window id（整數）
--   2. 分頁的 tty（/dev/ttysNNN）
--
-- 找到時回傳 "ok"；找不到時回傳空字串。
-- 以視窗 ID 找到視窗，以 tty 找到分頁設為 selected tab，再把視窗帶到最前。
-- 以 Cmd+T 開的原生分頁在 AppleScript 中是獨立的視窗，把該視窗帶到最前即切換到該分頁。
on run argv
	set targetWinId to (item 1 of argv) as integer
	set targetTty to item 2 of argv

	tell application id "com.apple.Terminal"
		activate
		try
			set w to window id targetWinId
			set tabTtys to tty of tabs of w
		on error number errNum
			-- 視窗已關閉：回傳找不到；其他錯誤（例如權限）照常拋出。
			if errNum is -1728 then return ""
			error number errNum
		end try

		repeat with ti from 1 to count of tabTtys
			if ((item ti of tabTtys) as text) is targetTty then
				try
					if miniaturized of w then set miniaturized of w to false
				end try
				set selected tab of w to tab ti of w
				set index of w to 1
				activate
				return "ok"
			end if
		end repeat
	end tell

	return ""
end run

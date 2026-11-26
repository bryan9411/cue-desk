-- 探索 Terminal.app 所有分頁。
-- Terminal.app 未執行時直接回傳空字串，不進入 tell 區塊，避免把 App 啟動。
--
-- 每個分頁一筆紀錄，欄位依序為：
--   window id ␟ 視窗序號 ␟ left ␟ top ␟ right ␟ bottom ␟ 分頁序號 ␟ tty ␟ custom title
-- 欄位以 ASCII 31（Unit Separator）、紀錄以 ASCII 30（Record Separator）分隔。
-- 序號皆從 1 起算。設定視窗等沒有分頁的視窗會略過。
--
-- 批次取值：每個視窗只送 3 個 Apple Event（所有分頁的 tty、bounds、
-- 所有分頁的 custom title），不再逐一讀取每個分頁的屬性。
-- 以 Cmd+T 開的原生分頁在 AppleScript 中是獨立的視窗，分頁序號通常是 1。
-- 視窗以 `window id` 參照，避免視窗前後順序在取值之間變動而錯位；
-- 兩份清單長度不一致（取值之間增減了分頁）時略過該視窗，下一輪再取。
on run argv
	if not (application id "com.apple.Terminal" is running) then return ""

	set fieldSep to (ASCII character 31)
	set recordSep to (ASCII character 30)
	set outRecords to {}

	tell application id "com.apple.Terminal"
		set winIds to id of windows
	end tell

	set winIndex to 0
	repeat with winId in winIds
		set winIndex to winIndex + 1
		set winId to contents of winId
		try
			tell application id "com.apple.Terminal"
				set w to window id winId
				set tabTtys to tty of tabs of w
				if (count of tabTtys) > 0 then
					set b to bounds of w
					set tabTitles to custom title of tabs of w
				else
					set tabTitles to {}
				end if
			end tell
			if (count of tabTtys) > 0 and (count of tabTtys) is (count of tabTitles) then
				set winPrefix to {winId as text, winIndex as text, ¬
					(item 1 of b) as text, (item 2 of b) as text, ¬
					(item 3 of b) as text, (item 4 of b) as text}
				repeat with ti from 1 to count of tabTtys
					set rowFields to winPrefix & {ti as text, ¬
						(item ti of tabTtys) as text, ¬
						my titleText(item ti of tabTitles)}
					set end of outRecords to my joinList(rowFields, fieldSep)
				end repeat
			end if
		on error number errNum
			-- 視窗在取值期間被關閉、或是沒有分頁的視窗（例如設定視窗）：略過；
			-- 權限錯誤（-1743）照常拋出。
			if errNum is -1743 then error number errNum
		end try
	end repeat

	return my joinList(outRecords, recordSep)
end run

-- custom title 可能是 missing value，也可能含任意字元；把分隔字元換成空白，避免破壞紀錄。
on titleText(theTitle)
	if theTitle is missing value then return ""
	set theText to theTitle as text
	set saved to AppleScript's text item delimiters
	repeat with sep in {(ASCII character 30), (ASCII character 31)}
		set AppleScript's text item delimiters to (contents of sep)
		set parts to text items of theText
		set AppleScript's text item delimiters to " "
		set theText to parts as text
	end repeat
	set AppleScript's text item delimiters to saved
	return theText
end titleText

on joinList(theList, delim)
	set saved to AppleScript's text item delimiters
	set AppleScript's text item delimiters to delim
	set joined to theList as text
	set AppleScript's text item delimiters to saved
	return joined
end joinList

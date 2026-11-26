-- 探索 Terminal.app 所有分頁。
-- Terminal.app 未執行時直接回傳空字串，不進入 tell 區塊，避免把 App 啟動。
--
-- 每個分頁一筆紀錄，欄位依序為：
--   window id ␟ 視窗序號 ␟ left ␟ top ␟ right ␟ bottom ␟ 分頁序號 ␟ tty ␟ custom title
-- 欄位以 ASCII 31（Unit Separator）、紀錄以 ASCII 30（Record Separator）分隔。
-- 序號皆從 1 起算。設定視窗等沒有分頁的視窗會略過。
on run argv
	if not (application id "com.apple.Terminal" is running) then return ""

	set fieldSep to (ASCII character 31)
	set recordSep to (ASCII character 30)
	set outRecords to {}

	tell application id "com.apple.Terminal"
		set winIndex to 0
		repeat with w in windows
			set winIndex to winIndex + 1
			try
				set winTabs to tabs of w
			on error
				set winTabs to {}
			end try
			if (count of winTabs) > 0 then
				set winId to id of w
				set b to bounds of w
				set tabIndex to 0
				repeat with t in winTabs
					set tabIndex to tabIndex + 1
					try
						set customTitle to (custom title of t) as text
					on error
						set customTitle to ""
					end try
					set rowFields to {winId as text, winIndex as text, ¬
						(item 1 of b) as text, (item 2 of b) as text, ¬
						(item 3 of b) as text, (item 4 of b) as text, ¬
						tabIndex as text, (tty of t) as text, customTitle}
					set end of outRecords to my joinList(rowFields, fieldSep)
				end repeat
			end if
		end repeat
	end tell

	return my joinList(outRecords, recordSep)
end run

on joinList(theList, delim)
	set saved to AppleScript's text item delimiters
	set AppleScript's text item delimiters to delim
	set joined to theList as text
	set AppleScript's text item delimiters to saved
	return joined
end joinList

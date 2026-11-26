-- 探索 iTerm2 所有窗格（session）。
-- iTerm2 未執行時直接回傳空字串，不進入 tell 區塊，避免把 App 啟動。
--
-- 每個窗格一筆紀錄，欄位依序為：
--   window id ␟ 視窗序號 ␟ left ␟ top ␟ right ␟ bottom ␟ 分頁序號 ␟ unique ID ␟ tty ␟ name
-- 欄位以 ASCII 31（Unit Separator）、紀錄以 ASCII 30（Record Separator）分隔。
-- 序號皆從 1 起算。
--
-- 批次取值：每個視窗只送 4 個 Apple Event（bounds，以及所有分頁中窗格的
-- unique ID、tty、name 各一次），不再逐一讀取每個窗格的屬性。
-- 視窗以 `window id` 參照，避免視窗前後順序在取值之間變動而錯位；
-- 同一視窗三份清單的形狀不一致（取值之間增減了分頁或窗格）時略過該視窗，下一輪再取。
on run argv
	if not (application id "com.googlecode.iterm2" is running) then return ""

	set fieldSep to (ASCII character 31)
	set recordSep to (ASCII character 30)
	set outRecords to {}

	tell application id "com.googlecode.iterm2"
		set winIds to id of windows
	end tell

	set winIndex to 0
	repeat with winId in winIds
		set winIndex to winIndex + 1
		set winId to contents of winId
		try
			tell application id "com.googlecode.iterm2"
				set w to window id winId
				set b to bounds of w
				set tabIds to unique ID of sessions of tabs of w
				set tabTtys to tty of sessions of tabs of w
				set tabNames to name of sessions of tabs of w
			end tell
			if my sameShape(tabIds, tabTtys) and my sameShape(tabIds, tabNames) then
				set winPrefix to {winId as text, winIndex as text, ¬
					(item 1 of b) as text, (item 2 of b) as text, ¬
					(item 3 of b) as text, (item 4 of b) as text}
				repeat with ti from 1 to count of tabIds
					set sessIds to item ti of tabIds
					set sessTtys to item ti of tabTtys
					set sessNames to item ti of tabNames
					repeat with si from 1 to count of sessIds
						set rowFields to winPrefix & {ti as text, ¬
							(item si of sessIds) as text, ¬
							(item si of sessTtys) as text, ¬
							my stripSeparators((item si of sessNames) as text)}
						set end of outRecords to my joinList(rowFields, fieldSep)
					end repeat
				end repeat
			end if
		on error number errNum
			-- 視窗在取值期間被關閉（-1728）：略過，下一輪再取；其他錯誤（例如權限）照常拋出。
			if errNum is not -1728 then error number errNum
		end try
	end repeat

	return my joinList(outRecords, recordSep)
end run

-- 兩份「分頁 → 窗格」巢狀清單的分頁數與每個分頁的窗格數都相同。
on sameShape(listA, listB)
	if (count of listA) is not (count of listB) then return false
	repeat with i from 1 to count of listA
		if (count of (item i of listA)) is not (count of (item i of listB)) then return false
	end repeat
	return true
end sameShape

-- name 由 shell 或程式設定，可能含任意字元；把分隔字元換成空白，避免破壞紀錄。
on stripSeparators(theText)
	set saved to AppleScript's text item delimiters
	repeat with sep in {(ASCII character 30), (ASCII character 31)}
		set AppleScript's text item delimiters to (contents of sep)
		set parts to text items of theText
		set AppleScript's text item delimiters to " "
		set theText to parts as text
	end repeat
	set AppleScript's text item delimiters to saved
	return theText
end stripSeparators

on joinList(theList, delim)
	set saved to AppleScript's text item delimiters
	set AppleScript's text item delimiters to delim
	set joined to theList as text
	set AppleScript's text item delimiters to saved
	return joined
end joinList

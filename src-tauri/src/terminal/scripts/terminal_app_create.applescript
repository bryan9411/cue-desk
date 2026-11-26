-- 在 Terminal.app 開新視窗並送出指令。
-- 由使用者主動建立觸發，因此會 activate Terminal.app（未執行時會啟動）。
--
-- argv：
--   1. 要送進新分頁的指令（已在呼叫端跳脫，腳本只原樣轉送）
--
-- `do script` 不指定 `in` 時一律開新視窗。
-- 回傳：新視窗的 window id ␟ 新分頁的 tty（`/dev/ttysNNN`）。
--
-- 未執行時先 `launch`（不送開啟事件），避免 Terminal.app 冷啟動時額外開出一個空白視窗。
-- 新分頁的 tty 在 shell 啟動前可能還是空字串，因此最多等 2 秒。
on run argv
	set cmd to item 1 of argv

	if not (application id "com.apple.Terminal" is running) then
		tell application id "com.apple.Terminal" to launch
	end if

	tell application id "com.apple.Terminal"
		set t to do script cmd
		set tabTty to ""
		repeat 20 times
			set tabTty to (tty of t) as text
			if tabTty is not "" then exit repeat
			delay 0.1
		end repeat
		if tabTty is "" then error "無法取得新分頁的 tty"

		-- 分頁參照無法直接取得所屬視窗，改以 tty 找出含有它的視窗。
		set winId to missing value
		repeat with w in windows
			try
				if (tty of tabs of w) contains tabTty then
					set winId to id of w
					exit repeat
				end if
			end try
		end repeat
		if winId is missing value then error "找不到新分頁所在的視窗"

		activate
		set index of window id winId to 1
		return (winId as text) & (ASCII character 31) & tabTty
	end tell
end run

import { useEffect, useRef, useState } from "react";

/** 標籤長度上限，與設定檔驗證一致。 */
const LABEL_MAX_CHARS = 64;

interface Props {
  initial: string;
  /** 清空時會回到的預設名稱。 */
  placeholder: string;
  ariaLabel: string;
  /** 只在內容有變動時呼叫；空字串表示清除標籤。`byKey` 表示以 Enter／Esc 結束。 */
  onCommit: (value: string, byKey: boolean) => void;
  onCancel: (byKey: boolean) => void;
}

/** 行內重新命名：Enter 或失焦確認、Esc 取消。 */
export function RenameField({ initial, placeholder, ariaLabel, onCommit, onCancel }: Props) {
  const [value, setValue] = useState(initial);
  const inputRef = useRef<HTMLInputElement>(null);
  // Enter／Esc 之後元件卸載還會觸發一次 blur，用旗標避免重複送出。
  const done = useRef(false);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const finish = (commit: boolean, byKey: boolean) => {
    if (done.current) return;
    done.current = true;
    const next = value.trim();
    if (commit && next !== initial.trim()) onCommit(next, byKey);
    else onCancel(byKey);
  };

  return (
    <input
      ref={inputRef}
      type="text"
      value={value}
      maxLength={LABEL_MAX_CHARS}
      placeholder={placeholder}
      aria-label={ariaLabel}
      spellCheck={false}
      autoComplete="off"
      onChange={(e) => setValue(e.target.value)}
      onKeyDown={(e) => {
        // 輸入法選字中的 Enter 用來確認候選字，不是確認重新命名。
        if (e.nativeEvent.isComposing) return;
        if (e.key === "Enter") {
          e.preventDefault();
          finish(true, true);
        } else if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          finish(false, true);
        }
      }}
      onBlur={() => finish(true, false)}
      className="min-w-0 flex-1 rounded-sm border border-border-strong bg-base px-1 text-[13px] leading-5 text-primary outline-none placeholder:text-muted focus-visible:outline-2 focus-visible:outline-focus-ring"
    />
  );
}

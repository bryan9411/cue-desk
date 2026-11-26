import { Warning } from "./icons/Icons";

/** 同一工作區有兩個以上 AI 在執行時，顯示在工作區名稱旁的提醒。 */
export function WarningBadge({ count }: { count: number }) {
  const reason = `這個資料夾有 ${count} 個 AI 正在執行。同一資料夾只有一個分支，同時修改檔案可能互相覆蓋。`;
  return (
    <span
      role="img"
      aria-label={reason}
      title={reason}
      className="mr-1 flex shrink-0 items-center gap-0.5 text-[11px] leading-4 text-status-waiting"
    >
      <Warning size={12} />
      <span className="font-mono">{count} AI</span>
    </span>
  );
}

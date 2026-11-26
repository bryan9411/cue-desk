import { GitBranch, GitCommit } from "../icons/Icons";

interface Props {
  /** 分支顯示字串（Detached 時為 `HEAD <前 7 碼>`）；非 Git 時為 `null`。 */
  branch: string | null;
  detached: boolean;
}

/**
 * 工作區的分支徽章（唯讀）。三種狀態以形狀區分而不靠顏色：
 * 分支圖示＝在分支上、commit 圖示＝Detached HEAD、虛線外框無圖示＝非 Git。
 */
export function BranchBadge({ branch, detached }: Props) {
  let icon = null;
  let spoken = "";
  let hint: string;
  if (branch === null) {
    hint = "這個資料夾不在 Git 儲存庫中";
  } else if (detached) {
    icon = <GitCommit size={12} className="shrink-0" />;
    spoken = "未在分支上：";
    hint = `${branch}：目前沒有在任何分支上（切到特定 commit 或 rebase 進行中）`;
  } else {
    icon = <GitBranch size={12} className="shrink-0" />;
    spoken = "分支：";
    hint = branch;
  }

  return (
    <span
      title={hint}
      className={`flex min-w-0 max-w-full items-center gap-1 rounded-sm border border-border-subtle bg-branch-badge-bg px-1 font-mono text-[11px] leading-4 text-branch-badge-fg ${
        branch === null ? "border-dashed" : ""
      }`}
    >
      {icon}
      {spoken && <span className="sr-only">{spoken}</span>}
      <span className="truncate">{branch ?? "非 Git"}</span>
    </span>
  );
}

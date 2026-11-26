import { useState } from "react";
import { focusSession } from "../../ipc/api";
import type { CueError, SessionKind, SessionNode as Node } from "../../types";
import { StatusLight } from "../StatusLight";

function kindLabel(kind: SessionKind): string {
  return kind.type === "other" ? kind.name : kind.type;
}

interface Props {
  node: Node;
  /** 取代位置提示的第二行文字（未歸類區用來顯示 cwd）。 */
  detail?: string;
}

/** 工作階段節點：點擊聚焦終端機，錯誤以文字顯示在節點下方。 */
export function SessionNode({ node, detail }: Props) {
  const [error, setError] = useState<CueError | null>(null);
  const [pending, setPending] = useState(false);
  const exited = node.status === "exited";
  const kind = kindLabel(node.kind);

  const focus = () => {
    if (pending) return;
    setPending(true);
    focusSession(node.id)
      .then(() => setError(null))
      .catch((e: CueError) => setError(e))
      .finally(() => setPending(false));
  };

  return (
    <li className={exited ? "opacity-55" : undefined}>
      <button
        type="button"
        onClick={focus}
        aria-busy={pending || undefined}
        className="group grid w-full grid-cols-[10px_1fr] items-center gap-x-2.5 rounded-sm px-2 py-1.5 text-left hover:bg-surface focus-visible:outline-2 focus-visible:outline-focus-ring active:bg-selected"
      >
        <StatusLight status={node.status} />
        <span className="flex min-w-0 items-baseline gap-2">
          <span className="shrink-0 rounded-sm border border-border-subtle px-1 font-mono text-[11px] leading-4 text-muted">
            {kind}
          </span>
          {node.label !== null && <span className="truncate text-[13px] text-primary">{node.label}</span>}
        </span>
        <span
          className="col-start-2 truncate font-mono text-[11px] leading-4 text-muted"
          title={detail ?? node.locationHint}
        >
          {detail ?? node.locationHint}
        </span>
      </button>
      {error && (
        <p role="alert" className="ml-[30px] mr-2 mb-1 text-[12px] leading-4 text-status-waiting">
          {error.message}
        </p>
      )}
    </li>
  );
}

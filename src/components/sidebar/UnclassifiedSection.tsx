import { homeDir } from "@tauri-apps/api/path";
import { useEffect, useState } from "react";
import { ignoreSession, setAsWorkspace, toCueError } from "../../ipc/api";
import { selectUnclassified, useCue } from "../../store/useCue";
import type { SessionNode as Node } from "../../types";
import { Plus } from "../icons/Icons";
import { SessionNode } from "./SessionNode";

/** 把家目錄開頭的路徑縮寫成 `~`。 */
export function abbreviateHome(path: string, home: string | null): string {
  if (!home) return path;
  const base = home.endsWith("/") ? home.slice(0, -1) : home;
  if (path === base) return "~";
  if (path.startsWith(`${base}/`)) return `~${path.slice(base.length)}`;
  return path;
}

const actionClass =
  "flex items-center gap-1 rounded-sm px-1 py-0.5 text-[12px] leading-4 text-muted hover:bg-surface hover:text-primary focus-visible:outline-2 focus-visible:outline-focus-ring disabled:opacity-50";

/** 未歸類節點下方的兩個操作：以它的 cwd 建立工作區，或不再列在這裡。 */
function UnclassifiedActions({ node }: { node: Node }) {
  const setLastError = useCue((s) => s.setLastError);
  const [pending, setPending] = useState(false);

  const run = (task: () => Promise<unknown>) => {
    if (pending) return;
    setPending(true);
    task()
      .catch((e) => setLastError(toCueError(e)))
      .finally(() => setPending(false));
  };

  return (
    <div className="mb-1 ml-[26px] flex gap-2">
      <button
        type="button"
        disabled={pending || node.cwd === null}
        onClick={() => run(() => setAsWorkspace(node.id))}
        title={node.cwd ?? undefined}
        className={actionClass}
      >
        <Plus size={12} />
        設為新工作區
      </button>
      <button type="button" disabled={pending} onClick={() => run(() => ignoreSession(node.id))} className={actionClass}>
        忽略
      </button>
    </div>
  );
}

/** 「未歸類」區：不屬於任何工作區的終端機，顯示目前 cwd。沒有節點時不顯示。 */
export function UnclassifiedSection() {
  const nodes = useCue(selectUnclassified);
  const [home, setHome] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    homeDir()
      .then((dir) => active && setHome(dir))
      .catch(() => {
        // 取不到家目錄時顯示完整路徑。
      });
    return () => {
      active = false;
    };
  }, []);

  if (nodes.length === 0) return null;

  return (
    <section aria-labelledby="unclassified-heading" className="mt-5 border-t border-border-subtle pt-3">
      <h2 id="unclassified-heading" className="flex items-baseline justify-between px-1 pb-1 text-[12px] text-muted">
        <span>未歸類</span>
        <span className="pr-1 font-mono text-[11px]">{nodes.length}</span>
      </h2>
      <ul>
        {nodes.map((node) => (
          <SessionNode
            key={node.id}
            node={node}
            detail={node.cwd === null ? "無法取得目錄" : abbreviateHome(node.cwd, home)}
            withMenu={false}
          >
            <UnclassifiedActions node={node} />
          </SessionNode>
        ))}
      </ul>
    </section>
  );
}

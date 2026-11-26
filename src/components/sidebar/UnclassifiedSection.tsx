import { homeDir } from "@tauri-apps/api/path";
import { useEffect, useState } from "react";
import { selectUnclassified, useCue } from "../../store/useCue";
import { SessionNode } from "./SessionNode";

/** 把家目錄開頭的路徑縮寫成 `~`。 */
export function abbreviateHome(path: string, home: string | null): string {
  if (!home) return path;
  const base = home.endsWith("/") ? home.slice(0, -1) : home;
  if (path === base) return "~";
  if (path.startsWith(`${base}/`)) return `~${path.slice(base.length)}`;
  return path;
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
          />
        ))}
      </ul>
    </section>
  );
}

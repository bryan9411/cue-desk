import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { addWorkspace, toCueError } from "../../ipc/api";
import { selectWorkspaceGroups, useCue } from "../../store/useCue";
import { CueMark, Folder, Plus } from "../icons/Icons";
import { UnclassifiedSection } from "./UnclassifiedSection";
import { WorkspaceNode } from "./WorkspaceNode";

/** 側邊欄：頂部標誌與加入工作區、工作區樹、最底部的「未歸類」區。 */
export function Sidebar() {
  const groups = useCue(selectWorkspaceGroups);
  const loaded = useCue((s) => s.backend !== null);
  const lastError = useCue((s) => s.lastError);
  const setLastError = useCue((s) => s.setLastError);
  const [picking, setPicking] = useState(false);

  const pickWorkspace = async () => {
    if (picking) return;
    setPicking(true);
    try {
      const path = await open({ directory: true, multiple: false, title: "選擇工作區資料夾" });
      if (typeof path === "string") await addWorkspace(path);
    } catch (e) {
      setLastError(toCueError(e));
    } finally {
      setPicking(false);
    }
  };

  return (
    <div className="flex h-full flex-col bg-sidebar text-primary">
      <header
        data-tauri-drag-region
        className="flex items-center gap-2 px-3 pb-2 pt-3 [border-bottom:var(--header-rule)]"
      >
        <CueMark size={18} className="pointer-events-none shrink-0 text-accent" />
        <h1 className="pointer-events-none flex-1 font-heading text-title leading-none [font-weight:var(--heading-weight)]">
          Cue
        </h1>
        <button
          type="button"
          onClick={pickWorkspace}
          disabled={picking}
          aria-label="加入工作區"
          title="加入工作區"
          className="rounded-sm p-1 text-muted hover:bg-surface hover:text-primary focus-visible:outline-2 focus-visible:outline-focus-ring disabled:opacity-50"
        >
          <Plus size={18} />
        </button>
      </header>

      {lastError && (
        <div role="alert" className="flex items-start gap-2 border-b border-border-subtle px-3 py-2 text-[12px] leading-4">
          <p className="flex-1 text-status-waiting">{lastError.message}</p>
          <button
            type="button"
            onClick={() => setLastError(null)}
            className="shrink-0 rounded-sm text-muted underline underline-offset-2 hover:text-primary focus-visible:outline-2 focus-visible:outline-focus-ring"
          >
            關閉
          </button>
        </div>
      )}

      <nav aria-label="工作區" className="min-h-0 flex-1 overflow-y-auto px-2 pb-4">
        {loaded && groups.length === 0 ? (
          <div className="flex flex-col items-start gap-3 px-2 pt-6 text-[13px] leading-5 text-muted">
            <Folder size={22} />
            <p>加入專案資料夾後，在其中開啟的終端機會自動歸到它底下。</p>
            <button
              type="button"
              onClick={pickWorkspace}
              disabled={picking}
              className="flex items-center gap-1.5 rounded-sm border border-border-strong px-2.5 py-1 text-primary hover:bg-surface focus-visible:outline-2 focus-visible:outline-focus-ring disabled:opacity-50"
            >
              <Plus size={14} />
              加入工作區
            </button>
          </div>
        ) : (
          <ul>
            {groups.map(({ workspace, sessions }) => (
              <WorkspaceNode key={workspace.id} workspace={workspace} sessions={sessions} />
            ))}
          </ul>
        )}
        <UnclassifiedSection />
      </nav>
    </div>
  );
}

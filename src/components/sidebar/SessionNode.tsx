import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  closeSession,
  focusSession,
  removeNode,
  renameSession,
  reopenSession,
  splitZsh,
  toCueError,
  type SplitDirection,
} from "../../ipc/api";
import { selectCapabilities, useCue } from "../../store/useCue";
import type { SessionKind, SessionNode as Node } from "../../types";
import { ContextMenu, useContextMenu, type MenuEntry } from "../ContextMenu";
import { StatusLight } from "../StatusLight";
import { RenameField } from "./RenameField";

function kindLabel(kind: SessionKind): string {
  return kind.type === "other" ? kind.name : kind.type;
}

interface Props {
  node: Node;
  /** 取代位置提示的第二行文字（未歸類區用來顯示 cwd）。 */
  detail?: string;
  /** 是否提供右鍵選單；未歸類區只用列下方的按鈕操作。 */
  withMenu?: boolean;
  /** 顯示在節點下方的操作列。 */
  children?: ReactNode;
}

type Message = { tone: "error" | "info"; text: string };

/** 工作階段節點：點擊聚焦終端機，右鍵開啟操作選單；結果訊息顯示在節點下方。 */
export function SessionNode({ node, detail, withMenu = true, children }: Props) {
  const caps = useCue(selectCapabilities);
  const [message, setMessage] = useState<Message | null>(null);
  const [pending, setPending] = useState(false);
  const [renaming, setRenaming] = useState(false);
  // 重新命名送出後先顯示新名稱，直到快照帶回新的標籤。
  const [optimisticLabel, setOptimisticLabel] = useState<string | null | undefined>(undefined);
  const menu = useContextMenu();
  const rowRef = useRef<HTMLButtonElement>(null);
  // 以 Enter／Esc 結束重新命名時把焦點還給節點；點別處結束時不搶焦點。
  const refocusRow = useRef(false);

  useEffect(() => setOptimisticLabel(undefined), [node.label]);
  useEffect(() => {
    if (renaming || !refocusRow.current) return;
    refocusRow.current = false;
    rowRef.current?.focus();
  }, [renaming]);

  const endRename = (byKey: boolean) => {
    refocusRow.current = byKey;
    setRenaming(false);
  };

  const exited = node.status === "exited";
  const kind = kindLabel(node.kind);
  const label = optimisticLabel !== undefined ? optimisticLabel : node.label;
  const name = label ?? node.defaultName;

  /** 執行一個操作；回傳字串時當作提示顯示。 */
  const run = (task: () => Promise<string | null>) => {
    if (pending) return;
    setPending(true);
    setMessage(null);
    task()
      .then((notice) => setMessage(notice ? { tone: "info", text: notice } : null))
      .catch((e) => setMessage({ tone: "error", text: toCueError(e).message }))
      .finally(() => setPending(false));
  };

  const focus = () => run(() => focusSession(node.id).then(() => null));
  const split = (direction: SplitDirection) => run(() => splitZsh(node.id, direction));

  const commitRename = (value: string, byKey: boolean) => {
    endRename(byKey);
    setOptimisticLabel(value === "" ? null : value);
    run(() =>
      renameSession(node.id, value).then(
        () => null,
        (e) => {
          setOptimisticLabel(undefined);
          throw e;
        },
      ),
    );
  };

  const splitNote = caps?.split ? undefined : "終端機後備模式改開新視窗";
  const renameItem: MenuEntry = {
    type: "item",
    label: "重新命名",
    onSelect: () => setRenaming(true),
    note: caps?.rename ? undefined : "名稱只顯示在 Cue，終端機標題不變",
  };

  const entries: MenuEntry[] = exited
    ? [
        { type: "item", label: "重新開啟", onSelect: () => run(() => reopenSession(node.id).then(() => null)) },
        renameItem,
        { type: "separator" },
        {
          type: "item",
          label: "移除",
          note: "從清單移除，並忘掉它的名稱",
          onSelect: () => run(() => removeNode(node.id).then(() => null)),
        },
      ]
    : [
        { type: "item", label: "聚焦", onSelect: focus },
        renameItem,
        { type: "separator" },
        { type: "item", label: "在右側補開同工作區 Zsh", note: splitNote, onSelect: () => split("right") },
        { type: "item", label: "在下方補開同工作區 Zsh", note: splitNote, onSelect: () => split("below") },
        { type: "separator" },
        {
          type: "item",
          label: "關閉",
          disabled: !caps?.closePane,
          note: caps?.closePane ? undefined : "目前的終端機不支援從 Cue 關閉",
          onSelect: () => run(() => closeSession(node.id).then(() => null)),
        },
      ];

  const rowClass =
    "group grid w-full grid-cols-[10px_1fr] items-center gap-x-2.5 rounded-sm px-2 py-1.5 text-left";
  const kindTag = (
    <span className="shrink-0 rounded-sm border border-border-subtle px-1 font-mono text-[11px] leading-4 text-muted">
      {kind}
    </span>
  );
  const secondLine = (
    <span className="col-start-2 truncate font-mono text-[11px] leading-4 text-muted" title={detail ?? node.locationHint}>
      {detail ?? node.locationHint}
    </span>
  );

  return (
    <li className={exited && !renaming ? "opacity-55" : undefined}>
      {renaming ? (
        <div className={`${rowClass} bg-surface`}>
          <StatusLight status={node.status} />
          <span className="flex min-w-0 items-center gap-2">
            {kindTag}
            <RenameField
              initial={node.label ?? ""}
              placeholder={node.defaultName}
              ariaLabel={`重新命名 ${name}`}
              onCommit={commitRename}
              onCancel={endRename}
            />
          </span>
          {secondLine}
        </div>
      ) : (
        <button
          ref={rowRef}
          type="button"
          onClick={focus}
          {...(withMenu ? menu.triggerProps : {})}
          aria-busy={pending || undefined}
          className={`${rowClass} hover:bg-surface focus-visible:outline-2 focus-visible:outline-focus-ring active:bg-selected`}
        >
          <StatusLight status={node.status} />
          <span className="flex min-w-0 items-baseline gap-2">
            {kindTag}
            {label !== null && <span className="truncate text-[13px] text-primary">{label}</span>}
          </span>
          {secondLine}
        </button>
      )}
      {children}
      {message && (
        <p
          role={message.tone === "error" ? "alert" : "status"}
          className={`ml-[30px] mr-2 mb-1 text-[12px] leading-4 ${
            message.tone === "error" ? "text-status-waiting" : "text-muted"
          }`}
        >
          {message.text}
        </p>
      )}
      {withMenu && menu.anchor && (
        <ContextMenu anchor={menu.anchor} title={name} entries={entries} onClose={menu.close} />
      )}
    </li>
  );
}

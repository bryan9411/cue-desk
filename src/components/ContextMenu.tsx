// 自繪右鍵選單：滑鼠位置開啟（鍵盤以 Shift+F10 從節點下方開啟），方向鍵移動、Enter 執行、Esc 關閉。
// 停用項目仍可用方向鍵停留，讓螢幕閱讀器讀得到停用原因；只是不能執行。
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
} from "react"
import { createPortal } from "react-dom"

interface MenuItem {
  type: "item"
  label: string
  onSelect: () => void
  disabled?: boolean
  /** 項目下方的小字：停用原因，或執行結果與預期不同時的說明。 */
  note?: string
}

export type MenuEntry = MenuItem | { type: "separator" }

export interface MenuAnchor {
  x: number
  y: number
}

/** 選單離視窗邊緣的最小距離（px）。 */
const EDGE = 8

/** 鍵盤開啟選單的按鍵：Shift+F10 或鍵盤上的選單鍵。 */
function isContextMenuKey(e: ReactKeyboardEvent): boolean {
  return (e.shiftKey && e.key === "F10") || e.key === "ContextMenu"
}

function isEnabledItem(el: HTMLButtonElement | null): el is HTMLButtonElement {
  return el !== null && el.getAttribute("aria-disabled") !== "true"
}

/** 方向鍵移動焦點後的位置；目前沒有焦點時從第一項開始，超出頭尾時循環。 */
function nextFocusIndex(current: number, count: number, step: number | "first" | "last"): number {
  if (step === "first") return 0
  if (step === "last") return count - 1
  if (current === -1) return 0
  return (current + step + count) % count
}

/** 量測後的位置：右側或下方放不下時往左／往上翻，避免超出視窗。 */
function fitInViewport(anchor: MenuAnchor, width: number, height: number): MenuAnchor {
  const maxX = window.innerWidth - width - EDGE
  const maxY = window.innerHeight - height - EDGE
  const x = Math.max(EDGE, Math.min(anchor.x, maxX))
  const y = anchor.y > maxY ? Math.max(EDGE, anchor.y - height) : anchor.y
  return { x, y }
}

/** 量測完成前先藏起來放在錨點，避免畫面閃一下出現在錯的位置。 */
function menuStyle(pos: MenuAnchor | null, anchor: MenuAnchor): CSSProperties {
  if (!pos) return { left: anchor.x, top: anchor.y, visibility: "hidden" }
  return { left: pos.x, top: pos.y, visibility: "visible" }
}

function preventNativeMenu(e: ReactMouseEvent) {
  e.preventDefault()
}

/** 滑鼠移過項目時讓它取得焦點，鍵盤與滑鼠的反白才會一致。 */
function focusOnHover(e: ReactPointerEvent<HTMLButtonElement>) {
  if (document.activeElement !== e.currentTarget) e.currentTarget.focus()
}

/**
 * 管理一個節點的選單開關。把 `triggerProps` 掛在節點上；
 * 以鍵盤或執行項目關閉時把焦點還給節點，點選單外面關閉時不搶焦點。
 */
export function useContextMenu() {
  const [anchor, setAnchor] = useState<MenuAnchor | null>(null)
  const triggerRef = useRef<HTMLElement | null>(null)

  const onContextMenu = useCallback((e: ReactMouseEvent<HTMLElement>) => {
    e.preventDefault()
    triggerRef.current = e.currentTarget
    setAnchor({ x: e.clientX, y: e.clientY })
  }, [])

  const onKeyDown = useCallback((e: ReactKeyboardEvent<HTMLElement>) => {
    if (!isContextMenuKey(e)) return
    e.preventDefault()
    const rect = e.currentTarget.getBoundingClientRect()
    triggerRef.current = e.currentTarget
    setAnchor({ x: rect.left + 24, y: rect.bottom })
  }, [])

  const close = useCallback((restoreFocus: boolean) => {
    setAnchor(null)
    if (restoreFocus) triggerRef.current?.focus()
  }, [])

  return { anchor, close, triggerProps: { onContextMenu, onKeyDown } }
}

interface Props {
  anchor: MenuAnchor
  /** 操作對象的名稱，顯示在選單頂端並作為選單的無障礙名稱。 */
  title: string
  entries: MenuEntry[]
  onClose: (restoreFocus: boolean) => void
}

export function ContextMenu({ anchor, title, entries, onClose }: Props) {
  const menuRef = useRef<HTMLDivElement>(null)
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([])
  const [pos, setPos] = useState<MenuAnchor | null>(null)

  useLayoutEffect(() => {
    const menu = menuRef.current
    if (!menu) return
    const { width, height } = menu.getBoundingClientRect()
    setPos(fitInViewport(anchor, width, height))
  }, [anchor])

  // 第一次定位完成後聚焦第一個可執行的項目。
  const focused = useRef(false)
  useEffect(() => {
    if (!pos || focused.current) return
    focused.current = true
    itemRefs.current.find(isEnabledItem)?.focus()
  }, [pos])

  // 點選單外面、視窗失焦、縮放或捲動時關閉，且不搶回焦點。
  useEffect(() => {
    function dismissOutside(e: Event) {
      if (!menuRef.current?.contains(e.target as Node)) onClose(false)
    }
    function dismiss() {
      onClose(false)
    }
    const listeners: [EventTarget, string, (e: Event) => void, boolean][] = [
      [document, "pointerdown", dismissOutside, true],
      [window, "blur", dismiss, false],
      [window, "resize", dismiss, false],
      [window, "scroll", dismiss, true],
    ]
    for (const [target, type, listener, capture] of listeners) target.addEventListener(type, listener, capture)
    return () => {
      for (const [target, type, listener, capture] of listeners) target.removeEventListener(type, listener, capture)
    }
  }, [onClose])

  function moveFocus(step: number | "first" | "last") {
    const list = itemRefs.current.filter((el): el is HTMLButtonElement => el !== null)
    if (list.length === 0) return
    const current = list.indexOf(document.activeElement as HTMLButtonElement)
    list[nextFocusIndex(current, list.length, step)].focus()
  }

  function selectEntry(entry: MenuItem) {
    if (entry.disabled) return
    onClose(true)
    entry.onSelect()
  }

  /** 依渲染順序記錄每個項目的 DOM，方向鍵移動時使用。 */
  function registerItem(index: number) {
    return (el: HTMLButtonElement | null) => {
      itemRefs.current[index] = el
    }
  }

  function handleKeyDown(e: ReactKeyboardEvent<HTMLDivElement>) {
    switch (e.key) {
      case "ArrowDown":
        moveFocus(1)
        break
      case "ArrowUp":
        moveFocus(-1)
        break
      case "Home":
        moveFocus("first")
        break
      case "End":
        moveFocus("last")
        break
      case "Escape":
      case "Tab":
        onClose(true)
        break
      default:
        return
    }
    e.preventDefault()
    e.stopPropagation()
  }

  let itemIndex = 0
  itemRefs.current = []

  return createPortal(
    <div
      ref={menuRef}
      role="menu"
      aria-label={title}
      onKeyDown={handleKeyDown}
      onContextMenu={preventNativeMenu}
      style={menuStyle(pos, anchor)}
      className="fixed z-50 min-w-52 max-w-68 rounded-sm border border-border-strong bg-base py-1 text-[13px] text-primary shadow-panel"
    >
      <p
        aria-hidden="true"
        className="mb-1 truncate border-b border-border-subtle px-3 pb-1.5 pt-0.5 font-heading text-[13px] text-muted [font-weight:var(--heading-weight)]"
      >
        {title}
      </p>
      {entries.map((entry, i) => {
        if (entry.type === "separator") {
          return <div key={`sep-${i}`} role="separator" className="my-1 border-t border-border-subtle" />
        }
        const index = itemIndex++
        return (
          <button
            key={entry.label}
            ref={registerItem(index)}
            type="button"
            role="menuitem"
            tabIndex={-1}
            aria-disabled={entry.disabled || undefined}
            onPointerMove={focusOnHover}
            onClick={() => selectEntry(entry)}
            className="flex w-full flex-col items-start px-3 py-1.5 text-left outline-none focus:bg-selected aria-disabled:cursor-default aria-disabled:text-muted"
          >
            <span>{entry.label}</span>
            {entry.note && <span className="text-[11px] leading-4 text-muted">{entry.note}</span>}
          </button>
        )
      })}
    </div>,
    document.body,
  )
}

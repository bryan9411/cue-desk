// 工作階段狀態燈號（圓點）：
// running／waiting／done 實心、idle 空心、exited 虛線空心，色弱者也能以形狀區分。
import type { SVGProps } from "react"
import type { Status } from "../types"

const STATUS_TEXT: Record<Status, string> = {
  idle: "閒置",
  running: "執行中",
  waiting: "等待回應",
  done: "已完成",
  exited: "已結束",
}

const STATUS_COLOR: Record<Status, string> = {
  idle: "text-status-idle",
  running: "text-status-running",
  waiting: "text-status-waiting",
  done: "text-status-done",
  exited: "text-status-exited",
}

const SOLID: SVGProps<SVGCircleElement> = { r: 4.25, fill: "currentColor", strokeWidth: 0 }
const HOLLOW: SVGProps<SVGCircleElement> = { r: 3.75, fill: "none", strokeWidth: 1.5 }
const DASHED: SVGProps<SVGCircleElement> = { ...HOLLOW, strokeDasharray: "1.6 1.4" }

const STATUS_SHAPE: Record<Status, SVGProps<SVGCircleElement>> = {
  idle: HOLLOW,
  running: SOLID,
  waiting: SOLID,
  done: SOLID,
  exited: DASHED,
}

export function statusText(status: Status): string {
  return STATUS_TEXT[status]
}

export function StatusLight({ status, size = 10 }: { status: Status; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 10 10"
      role="img"
      aria-label={STATUS_TEXT[status]}
      className={`shrink-0 ${STATUS_COLOR[status]}`}
    >
      <title>{STATUS_TEXT[status]}</title>
      <circle cx="5" cy="5" stroke="currentColor" {...STATUS_SHAPE[status]} />
    </svg>
  )
}

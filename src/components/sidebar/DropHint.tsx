import { Terminal, Warning } from "../icons/Icons"

type Tone = "accept" | "reject"

interface Props {
  /** `accept`：放開後會處理；`reject`：放開也不會處理，文字為原因。 */
  tone: Tone
  text: string
  className?: string
}

const TONE_STYLE: Record<Tone, { Icon: typeof Terminal; color: string }> = {
  accept: { Icon: Terminal, color: "text-accent" },
  reject: { Icon: Warning, color: "text-status-waiting" },
}

/** 拖曳懸停在落點上時的一行提示。 */
export function DropHint({ tone, text, className = "" }: Props) {
  const { Icon, color } = TONE_STYLE[tone]
  return (
    <p role="status" className={`flex items-start gap-1.5 text-[12px] leading-4 ${color} ${className}`}>
      <Icon size={12} className="mt-0.5 shrink-0" />
      <span>{text}</span>
    </p>
  )
}

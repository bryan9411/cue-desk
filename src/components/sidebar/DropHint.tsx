import { Terminal, Warning } from "../icons/Icons";

interface Props {
  /** `accept`：放開後會處理；`reject`：放開也不會處理，文字為原因。 */
  tone: "accept" | "reject";
  text: string;
  className?: string;
}

/** 拖曳懸停在落點上時的一行提示。 */
export function DropHint({ tone, text, className = "" }: Props) {
  const Icon = tone === "accept" ? Terminal : Warning;
  return (
    <p
      role="status"
      className={`flex items-start gap-1.5 text-[12px] leading-4 ${
        tone === "accept" ? "text-accent" : "text-status-waiting"
      } ${className}`}
    >
      <Icon size={12} className="mt-0.5 shrink-0" />
      <span>{text}</span>
    </p>
  );
}

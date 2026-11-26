// 極簡細線內嵌 SVG 圖示：24 viewBox、stroke="currentColor"、不填色、不使用 emoji。
import type { ReactNode, SVGProps } from "react";

export type IconProps = Omit<SVGProps<SVGSVGElement>, "children"> & {
  /** 邊長（px），預設 16。 */
  size?: number;
};

function Icon({ size = 16, children, ...rest }: IconProps & { children: ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      {children}
    </svg>
  );
}

export function Plus(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M12 5v14M5 12h14" />
    </Icon>
  );
}

export function ChevronRight(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M9.5 6l6 6-6 6" />
    </Icon>
  );
}

export function ChevronDown(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M6 9.5l6 6 6-6" />
    </Icon>
  );
}

export function Folder(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M3.5 7.5v10a1.5 1.5 0 0 0 1.5 1.5h14a1.5 1.5 0 0 0 1.5-1.5V9.5A1.5 1.5 0 0 0 19 8h-7l-2-2.5H5A1.5 1.5 0 0 0 3.5 7v.5z" />
    </Icon>
  );
}

export function Terminal(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="3.5" y="4.5" width="17" height="15" rx="1.5" />
      <path d="M7.5 10l3 2.5-3 2.5M12.5 15.5h4" />
    </Icon>
  );
}

/** Cue 標誌：一筆斜線指揮棒與落點。 */
export function CueMark(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M6 19L18 5" />
      <circle cx="18" cy="5" r="1" />
      <path d="M4 15.5c1.5.5 2.5 1.5 3 3" />
    </Icon>
  );
}

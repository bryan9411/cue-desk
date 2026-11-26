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

/** 警示三角形。 */
export function Warning(props: IconProps) {
  return (
    <Icon {...props}>
      <path d="M10.7 4.6a1.5 1.5 0 0 1 2.6 0l7.4 13a1.5 1.5 0 0 1-1.3 2.2H4.6a1.5 1.5 0 0 1-1.3-2.2z" />
      <path d="M12 9.5v4.5M12 16.75v.25" />
    </Icon>
  );
}

/** Git 分支：主線與一條分出的支線。 */
export function GitBranch(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="7" cy="5.5" r="2" />
      <circle cx="7" cy="18.5" r="2" />
      <circle cx="17" cy="7" r="2" />
      <path d="M7 7.5v9M17 9c0 4.5-4.5 6-8.5 8.1" />
    </Icon>
  );
}

/** 單一 commit：線上的一個節點，用於 Detached HEAD。 */
export function GitCommit(props: IconProps) {
  return (
    <Icon {...props}>
      <circle cx="12" cy="12" r="3.5" />
      <path d="M3.5 12h5M15.5 12h5" />
    </Icon>
  );
}

// 開啟方式示意：同一個視窗外框，以內部分隔線區分。

export function LayoutWindow(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="3.5" y="5" width="17" height="14" rx="1.5" />
      <path d="M3.5 8.5h17" />
    </Icon>
  );
}

export function LayoutTab(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="3.5" y="5" width="17" height="14" rx="1.5" />
      <path d="M3.5 8.5h17M9.5 5v3.5M15 5v3.5" />
    </Icon>
  );
}

export function LayoutSplitRows(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="3.5" y="5" width="17" height="14" rx="1.5" />
      <path d="M3.5 12h17" />
    </Icon>
  );
}

export function LayoutSplitColumns(props: IconProps) {
  return (
    <Icon {...props}>
      <rect x="3.5" y="5" width="17" height="14" rx="1.5" />
      <path d="M12 5v14" />
    </Icon>
  );
}

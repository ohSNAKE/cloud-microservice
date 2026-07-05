interface SidebarToggleIconProps {
  className?: string;
}

/** 侧栏展开/收起图标（圆角矩形 + 左侧菜单线） */
export default function SidebarToggleIcon({ className }: SidebarToggleIconProps) {
  return (
    <svg
      className={className}
      viewBox="0 0 16 16"
      width="1em"
      height="1em"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      aria-hidden
    >
      <rect
        x="1.25"
        y="2.25"
        width="13.5"
        height="11.5"
        rx="2"
        stroke="currentColor"
        strokeWidth="1.25"
      />
      <path d="M5.75 2.25v11.5" stroke="currentColor" strokeWidth="1.25" />
      <path
        d="M2.75 5.5h2.25M2.75 8h2.25M2.75 10.5h2.25"
        stroke="currentColor"
        strokeWidth="1.1"
        strokeLinecap="round"
      />
    </svg>
  );
}

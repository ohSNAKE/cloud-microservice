import type { MouseEvent, ReactNode } from "react";

export async function startWindowDrag(e: MouseEvent) {
  if (e.button !== 0) return;
  const target = e.target as HTMLElement;
  if (target.closest("button, a, input, textarea, select, [data-no-drag], .ant-segmented, .ant-switch")) {
    return;
  }
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    await getCurrentWindow().startDragging();
  } catch {
    /* 浏览器预览环境 */
  }
}

interface WindowDragRegionProps {
  children: ReactNode;
  className?: string;
  /** 设为 true 时该区域不参与拖拽（如按钮组） */
  noDrag?: boolean;
}

export default function WindowDragRegion({ children, className, noDrag }: WindowDragRegionProps) {
  if (noDrag) {
    return (
      <div className={className} data-no-drag onMouseDown={(e) => e.stopPropagation()}>
        {children}
      </div>
    );
  }

  return (
    <div
      className={className ? `window-drag-region ${className}` : "window-drag-region"}
      data-tauri-drag-region
      onMouseDown={startWindowDrag}
    >
      {children}
    </div>
  );
}

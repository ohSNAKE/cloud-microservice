import { Empty, type EmptyProps } from "antd";

interface EmptyPlaceholderProps {
  description?: string;
  children?: EmptyProps["children"];
}

export default function EmptyPlaceholder({ description, children }: EmptyPlaceholderProps) {
  return (
    <div className="empty-placeholder">
      <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={description}>
        {children}
      </Empty>
    </div>
  );
}

import type { ReactNode } from "react";

interface PageHeaderProps {
  title?: string;
  subtitle?: string;
  actions?: ReactNode;
}

export default function PageHeader({ title, subtitle, actions }: PageHeaderProps) {
  if (!title && !subtitle && !actions) return null;
  return (
    <div className="page-header">
      {(title || subtitle) && (
        <div className="page-header__main">
          {title && <h2>{title}</h2>}
          {subtitle && <p>{subtitle}</p>}
        </div>
      )}
      {actions && <div className="page-header__actions">{actions}</div>}
    </div>
  );
}

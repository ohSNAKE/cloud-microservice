import { DEFAULT_CATEGORY_ICON, EXPENSE_ICON_OPTIONS, INCOME_ICON_OPTIONS, renderIcon } from "./iconRegistry";

interface IconPickerProps {
  value?: string;
  onChange?: (value: string) => void;
  type?: "income" | "expense";
}

export default function IconPicker({ value, onChange, type = "expense" }: IconPickerProps) {
  const options = type === "income" ? INCOME_ICON_OPTIONS : EXPENSE_ICON_OPTIONS;
  const selected = value ?? DEFAULT_CATEGORY_ICON;

  return (
    <div className="icon-picker">
      {options.map((opt) => (
        <button
          key={opt.key}
          type="button"
          className={`icon-picker__item${selected === opt.key ? " is-selected" : ""}`}
          title={opt.label}
          aria-label={opt.label}
          onClick={() => onChange?.(opt.key)}
        >
          {renderIcon(opt.key, 16)}
        </button>
      ))}
    </div>
  );
}

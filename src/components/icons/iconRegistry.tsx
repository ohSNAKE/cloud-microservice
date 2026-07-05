import type { ReactNode } from "react";
import {
  AlipayCircleOutlined,
  BankOutlined,
  BookOutlined,
  CarOutlined,
  CoffeeOutlined,
  DollarOutlined,
  GiftOutlined,
  HomeOutlined,
  LineChartOutlined,
  MedicineBoxOutlined,
  MoneyCollectOutlined,
  PlaySquareOutlined,
  PushpinOutlined,
  RiseOutlined,
  ShoppingOutlined,
  TagOutlined,
  WalletOutlined,
  WechatOutlined,
} from "@ant-design/icons";

export const DEFAULT_CATEGORY_ICON = "pushpin";

/** 旧 emoji 到 icon key 的映射，兼容已有数据 */
export const EMOJI_TO_ICON_KEY: Record<string, string> = {
  "🍜": "dining",
  "🚗": "transport",
  "🛒": "shopping",
  "🏠": "housing",
  "🎮": "entertainment",
  "💊": "medical",
  "📚": "education",
  "📌": "pushpin",
  "💰": "salary",
  "🎁": "gift",
  "📈": "investment",
  "💵": "cash-income",
  "💙": "alipay",
  "💚": "wechat",
  "🏦": "bank",
};

export const EXPENSE_ICON_OPTIONS = [
  { key: "dining", label: "餐饮" },
  { key: "transport", label: "交通" },
  { key: "shopping", label: "购物" },
  { key: "housing", label: "住房" },
  { key: "entertainment", label: "娱乐" },
  { key: "medical", label: "医疗" },
  { key: "education", label: "教育" },
  { key: "pushpin", label: "其他" },
] as const;

export const INCOME_ICON_OPTIONS = [
  { key: "salary", label: "工资" },
  { key: "gift", label: "奖金" },
  { key: "investment", label: "理财" },
  { key: "cash-income", label: "现金" },
  { key: "pushpin", label: "其他" },
] as const;

const ICON_COMPONENTS: Record<string, typeof TagOutlined> = {
  dining: CoffeeOutlined,
  transport: CarOutlined,
  shopping: ShoppingOutlined,
  housing: HomeOutlined,
  entertainment: PlaySquareOutlined,
  medical: MedicineBoxOutlined,
  education: BookOutlined,
  pushpin: PushpinOutlined,
  salary: MoneyCollectOutlined,
  gift: GiftOutlined,
  investment: RiseOutlined,
  "cash-income": DollarOutlined,
  cash: WalletOutlined,
  bank: BankOutlined,
  alipay: AlipayCircleOutlined,
  wechat: WechatOutlined,
  broker: LineChartOutlined,
  tag: TagOutlined,
};

export function normalizeIconKey(raw: string | null | undefined): string {
  if (!raw) return DEFAULT_CATEGORY_ICON;
  if (ICON_COMPONENTS[raw]) return raw;
  return EMOJI_TO_ICON_KEY[raw] ?? DEFAULT_CATEGORY_ICON;
}

export function renderIcon(
  raw: string | null | undefined,
  size = 14,
  className?: string,
): ReactNode {
  const key = normalizeIconKey(raw);
  const Icon = ICON_COMPONENTS[key] ?? TagOutlined;
  return <Icon style={{ fontSize: size }} className={className} />;
}

interface CategoryIconProps {
  icon?: string | null;
  size?: number;
  className?: string;
}

export function CategoryIcon({ icon, size = 14, className }: CategoryIconProps) {
  return renderIcon(icon, size, className);
}

interface CategoryLabelProps {
  icon?: string | null;
  name: string;
  iconSize?: number;
}

export function CategoryLabel({ icon, name, iconSize = 14 }: CategoryLabelProps) {
  return (
    <span className="category-label">
      <CategoryIcon icon={icon} size={iconSize} />
      <span>{name}</span>
    </span>
  );
}

interface AccountTypeIconProps {
  type: string;
  size?: number;
  className?: string;
}

export function AccountTypeIcon({ type, size = 14, className }: AccountTypeIconProps) {
  return renderIcon(type, size, className);
}

interface AccountTypeLabelProps {
  type: string;
  label: string;
  iconSize?: number;
}

export function AccountTypeLabel({ type, label, iconSize = 14 }: AccountTypeLabelProps) {
  return (
    <span className="category-label">
      <AccountTypeIcon type={type} size={iconSize} />
      <span>{label}</span>
    </span>
  );
}

export function categorySelectOption(category: { id: number; icon: string; name: string }) {
  return {
    label: <CategoryLabel icon={category.icon} name={category.name} />,
    value: category.id,
  };
}

export function accountTypeSelectOption(type: { value: string; label: string; icon: string }) {
  return {
    label: <AccountTypeLabel type={type.icon} label={type.label} />,
    value: type.value,
  };
}

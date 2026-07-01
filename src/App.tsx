import { useEffect, useMemo, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes, useLocation, useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import {
  BarChartOutlined,
  DashboardOutlined,
  LineChartOutlined,
  PlusOutlined,
  SettingOutlined,
  WalletOutlined,
} from "@ant-design/icons";
import { Button, Layout, Menu, Space, Typography } from "antd";
import DashboardPage from "./pages/Dashboard";
import TransactionsPage from "./pages/Transactions";
import HoldingsPage from "./pages/Holdings";
import ReportsPage from "./pages/Reports";
import SettingsPage from "./pages/Settings";
import QuickAddModal from "./components/QuickAddModal";
import ThemeSwitcher from "./components/theme/ThemeSwitcher";
import { QuickAddProvider } from "./context/QuickAddContext";

const { Header, Sider, Content } = Layout;

const menuItems = [
  { key: "/", icon: <DashboardOutlined />, label: "财务总览" },
  { key: "/transactions", icon: <WalletOutlined />, label: "记账" },
  { key: "/holdings", icon: <LineChartOutlined />, label: "投资持仓" },
  { key: "/reports", icon: <BarChartOutlined />, label: "报表分析" },
  { key: "/settings", icon: <SettingOutlined />, label: "设置" },
];

const routeMeta: Record<string, { title: string; subtitle: string }> = {
  "/": { title: "财务总览", subtitle: "一眼看清资产、收支与预算" },
  "/transactions": { title: "记账", subtitle: "追踪每一笔收入与支出" },
  "/holdings": { title: "投资持仓", subtitle: "股票基金市值与盈亏" },
  "/reports": { title: "报表分析", subtitle: "分类统计、预算与趋势" },
  "/settings": { title: "设置", subtitle: "AI 记账、行情与数据备份" },
};

function shortcutLabel() {
  const isMac = navigator.platform.toLowerCase().includes("mac");
  return isMac ? "⌘N" : "Ctrl+N";
}

function AppLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const [collapsed, setCollapsed] = useState(false);
  const [quickAddOpen, setQuickAddOpen] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const selectedKey = useMemo(() => location.pathname || "/", [location.pathname]);
  const meta = routeMeta[selectedKey] ?? routeMeta["/"];

  const openQuickAdd = () => setQuickAddOpen(true);

  useEffect(() => {
    const unlisten = listen("quick-add-transaction", () => setQuickAddOpen(true));
    const onKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "n") {
        e.preventDefault();
        setQuickAddOpen(true);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  return (
    <QuickAddProvider openQuickAdd={openQuickAdd}>
      <Layout className="app-shell">
        <Sider
          className="app-sider"
          collapsible
          collapsed={collapsed}
          onCollapse={setCollapsed}
          theme="light"
          width={228}
          collapsedWidth={64}
        >
          <div className="app-sider__brand">
            <div className="app-sider__logo">财</div>
            {!collapsed && <span className="app-sider__title">财记</span>}
          </div>
          <Menu
            className="app-menu"
            theme="light"
            mode="inline"
            selectedKeys={[selectedKey]}
            items={menuItems}
            onClick={({ key }) => navigate(key)}
          />
        </Sider>
        <Layout className="app-main">
          <Header className="app-header">
            <div className="app-header__drag" data-tauri-drag-region>
              <Typography.Text className="app-header__title">{meta.title}</Typography.Text>
              <Typography.Paragraph className="app-header__subtitle" style={{ margin: 0 }}>
                {meta.subtitle}
              </Typography.Paragraph>
            </div>
            <Space wrap>
              <ThemeSwitcher size="small" />
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {shortcutLabel()} 智能记账
              </Typography.Text>
              <Button type="primary" icon={<PlusOutlined />} onClick={openQuickAdd}>
                智能记账
              </Button>
            </Space>
          </Header>
          <Content className="app-content">
            <div className="app-content-inner" key={refreshKey}>
              <Routes>
                <Route path="/" element={<DashboardPage />} />
                <Route path="/transactions" element={<TransactionsPage />} />
                <Route path="/holdings" element={<HoldingsPage />} />
                <Route path="/reports" element={<ReportsPage />} />
                <Route path="/settings" element={<SettingsPage />} />
                <Route path="*" element={<Navigate to="/" replace />} />
              </Routes>
            </div>
          </Content>
        </Layout>
        <QuickAddModal
          open={quickAddOpen}
          onClose={() => setQuickAddOpen(false)}
          onSuccess={() => setRefreshKey((k) => k + 1)}
        />
      </Layout>
    </QuickAddProvider>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <AppLayout />
    </BrowserRouter>
  );
}

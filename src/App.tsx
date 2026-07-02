import { useEffect, useMemo, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes, useLocation, useNavigate } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import {
  BarChartOutlined,
  BankOutlined,
  DashboardOutlined,
  LineChartOutlined,
  SettingOutlined,
  WalletOutlined,
} from "@ant-design/icons";
import { Layout, Menu, Typography } from "antd";
import AppHeaderActions from "./components/layout/AppHeaderActions";
import DashboardPage from "./pages/Dashboard";
import AssetsPage from "./pages/Assets";
import TransactionsPage from "./pages/Transactions";
import HoldingsPage from "./pages/Holdings";
import ReportsPage from "./pages/Reports";
import SettingsPage from "./pages/Settings";
import QuickAddModal from "./components/QuickAddModal";
import { startWindowDrag } from "./components/layout/WindowDragRegion";
import { QuickAddProvider } from "./context/QuickAddContext";

const { Header, Sider, Content } = Layout;

const menuItems = [
  { key: "/", icon: <DashboardOutlined />, label: "财务总览" },
  { key: "/assets", icon: <BankOutlined />, label: "我的资产" },
  { key: "/transactions", icon: <WalletOutlined />, label: "记账" },
  { key: "/holdings", icon: <LineChartOutlined />, label: "投资持仓" },
  { key: "/reports", icon: <BarChartOutlined />, label: "报表分析" },
  { key: "/settings", icon: <SettingOutlined />, label: "设置" },
];

const routeMeta: Record<string, { title: string; subtitle: string }> = {
  "/": { title: "财务总览", subtitle: "一眼看清资产、收支与预算" },
  "/assets": { title: "我的资产", subtitle: "登记银行卡、支付宝、现金等个人余额" },
  "/transactions": { title: "记账", subtitle: "追踪每一笔收入与支出" },
  "/holdings": { title: "投资持仓", subtitle: "股票基金市值与盈亏" },
  "/reports": { title: "报表分析", subtitle: "分类统计、预算与趋势" },
  "/settings": { title: "设置", subtitle: "AI 记账、行情与数据备份" },
};


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
          trigger={null}
          theme="light"
          width={228}
          collapsedWidth={64}
        >
          <div
            className="app-sider__drag-strip window-drag-region"
            data-tauri-drag-region
            onMouseDown={startWindowDrag}
          />
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
          <Header
            className="app-header window-drag-region"
            data-tauri-drag-region
            onMouseDown={startWindowDrag}
          >
            <div className="app-header__main">
              <Typography.Text className="app-header__title">{meta.title}</Typography.Text>
              <Typography.Paragraph className="app-header__subtitle" style={{ margin: 0 }}>
                {meta.subtitle}
              </Typography.Paragraph>
            </div>
            <div className="app-header__actions" data-no-drag onMouseDown={(e) => e.stopPropagation()}>
              <AppHeaderActions
                collapsed={collapsed}
                onToggleSidebar={() => setCollapsed(!collapsed)}
                onQuickAdd={openQuickAdd}
              />
            </div>
          </Header>
          <Content className="app-content">
            <div className="app-content-inner" key={refreshKey}>
              <Routes>
                <Route path="/" element={<DashboardPage />} />
                <Route path="/assets" element={<AssetsPage />} />
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

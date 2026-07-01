import { useMemo, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes, useLocation, useNavigate } from "react-router-dom";
import {
  BarChartOutlined,
  DashboardOutlined,
  LineChartOutlined,
  SettingOutlined,
  WalletOutlined,
} from "@ant-design/icons";
import { Layout, Menu, Typography } from "antd";
import DashboardPage from "./pages/Dashboard";
import TransactionsPage from "./pages/Transactions";
import HoldingsPage from "./pages/Holdings";
import ReportsPage from "./pages/Reports";
import SettingsPage from "./pages/Settings";

const { Header, Sider, Content } = Layout;

const menuItems = [
  { key: "/", icon: <DashboardOutlined />, label: "财务总览" },
  { key: "/transactions", icon: <WalletOutlined />, label: "记账" },
  { key: "/holdings", icon: <LineChartOutlined />, label: "投资持仓" },
  { key: "/reports", icon: <BarChartOutlined />, label: "报表分析" },
  { key: "/settings", icon: <SettingOutlined />, label: "设置" },
];

function AppLayout() {
  const location = useLocation();
  const navigate = useNavigate();
  const [collapsed, setCollapsed] = useState(false);
  const selectedKey = useMemo(() => location.pathname || "/", [location.pathname]);

  return (
    <Layout style={{ minHeight: "100vh" }}>
      <Sider
        collapsible
        collapsed={collapsed}
        onCollapse={setCollapsed}
        theme="light"
        width={220}
        style={{ borderRight: "1px solid #eef2f7" }}
      >
        <div style={{ padding: "20px 16px", fontWeight: 700, fontSize: 18 }}>财记</div>
        <Menu
          mode="inline"
          selectedKeys={[selectedKey]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
        />
      </Sider>
      <Layout>
        <Header
          style={{
            background: "#fff",
            borderBottom: "1px solid #eef2f7",
            padding: "0 24px",
            display: "flex",
            alignItems: "center",
          }}
        >
          <Typography.Text type="secondary">
            管理资产 · 追踪支出 · 数据仅存本地
          </Typography.Text>
        </Header>
        <Content style={{ padding: 24 }}>
          <Routes>
            <Route path="/" element={<DashboardPage />} />
            <Route path="/transactions" element={<TransactionsPage />} />
            <Route path="/holdings" element={<HoldingsPage />} />
            <Route path="/reports" element={<ReportsPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </Content>
      </Layout>
    </Layout>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <AppLayout />
    </BrowserRouter>
  );
}

import React from "react";
import ReactDOM from "react-dom/client";
import { ConfigProvider } from "antd";
import zhCN from "antd/locale/zh_CN";
import dayjs from "dayjs";
import "dayjs/locale/zh-cn";
import App from "./App";
import "./styles/tokens.css";
import "./styles/global.css";

dayjs.locale("zh-cn");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ConfigProvider
      locale={zhCN}
      theme={{
        token: {
          colorPrimary: "#1677ff",
          colorBgLayout: "#f4f6fa",
          colorText: "#1d2939",
          colorTextSecondary: "#667085",
          borderRadius: 8,
          fontFamily:
            '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "PingFang SC", "Microsoft YaHei", sans-serif',
        },
        components: {
          Layout: {
            headerBg: "#ffffff",
            siderBg: "#ffffff",
            bodyBg: "#f4f6fa",
          },
          Menu: {
            itemBorderRadius: 8,
            itemMarginInline: 8,
            itemHeight: 40,
          },
          Card: {
            borderRadiusLG: 12,
          },
          Table: {
            headerBg: "#fafbfc",
          },
          Button: {
            borderRadius: 8,
          },
        },
      }}
    >
      <App />
    </ConfigProvider>
  </React.StrictMode>,
);

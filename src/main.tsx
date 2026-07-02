import React from "react";
import ReactDOM from "react-dom/client";
import dayjs from "dayjs";
import "dayjs/locale/zh-cn";
import App from "./App";
import { AppLockProvider } from "./context/AppLockContext";
import AppThemeProvider from "./components/theme/AppThemeProvider";
import "./styles/tokens.css";
import "./styles/global.css";

dayjs.locale("zh-cn");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AppThemeProvider>
      <AppLockProvider>
        <App />
      </AppLockProvider>
    </AppThemeProvider>
  </React.StrictMode>,
);

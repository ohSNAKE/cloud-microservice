# 财记 - 个人财务桌面应用

基于 **Tauri 2 + React + TypeScript + SQLite** 的跨平台桌面应用。

## 功能

- 记账：收入/支出记录、分类、账户管理
- 股票基金：持仓登记、盈亏计算
- 行情更新：手动刷新 + 定时自动更新（东方财富/天天基金接口）
- 财务总览：净资产、收支趋势、资产分布图表

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 前端 | React 19 + TypeScript + Ant Design + ECharts |
| 后端 | Rust（Tauri 命令） |
| 数据库 | SQLite（本地存储） |

## 开发

### 环境要求

- Node.js 18+
- Rust 1.85+
- 系统依赖见 [Tauri 前置条件](https://tauri.app/start/prerequisites/)

### 启动

```bash
npm install
npm run tauri dev
```

### 打包

```bash
npm run tauri build
```

## 数据说明

所有财务数据存储在本地 SQLite 数据库，路径为系统应用数据目录下的 `finance.db`。

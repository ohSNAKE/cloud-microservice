import { LockOutlined, PlusOutlined } from "@ant-design/icons";
import { Button, Space, Tooltip, message } from "antd";
import { useAppLock } from "../../context/AppLockContext";
import ThemeSwitcher from "../theme/ThemeSwitcher";

interface AppHeaderActionsProps {
  onQuickAdd: () => void;
}

function shortcutLabel() {
  const isMac = navigator.platform.toLowerCase().includes("mac");
  return isMac ? "⌘N" : "Ctrl+N";
}

export default function AppHeaderActions({ onQuickAdd }: AppHeaderActionsProps) {
  const { lockEnabled, lock } = useAppLock();

  const handleLock = () => {
    if (!lockEnabled) {
      message.info("请先在设置中开启应用锁");
      return;
    }
    lock();
  };

  return (
    <Space size={4}>
      <Tooltip title={lockEnabled ? "锁定应用" : "请先在设置中开启应用锁"}>
        <Button
          type="text"
          className="app-header__icon-btn"
          icon={<LockOutlined />}
          aria-label="锁定应用"
          onClick={handleLock}
        />
      </Tooltip>
      <ThemeSwitcher size="small" />
      <Tooltip title={`${shortcutLabel()} 智能记账`}>
        <Button
          type="text"
          className="app-header__icon-btn"
          icon={<PlusOutlined />}
          aria-label="智能记账"
          onClick={onQuickAdd}
        />
      </Tooltip>
    </Space>
  );
}

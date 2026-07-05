import { Spin } from "antd";

interface PageLoaderProps {
  tip?: string;
}

export default function PageLoader({ tip = "加载中..." }: PageLoaderProps) {
  return (
    <div className="page-loader">
      <Spin size="large" tip={tip} />
    </div>
  );
}

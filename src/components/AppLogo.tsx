interface AppLogoProps {
  size?: number;
  className?: string;
}

export default function AppLogo({ size = 32, className }: AppLogoProps) {
  return (
    <img
      src="/app-logo.png"
      alt="穷鬼"
      width={size}
      height={size}
      className={className}
      draggable={false}
    />
  );
}

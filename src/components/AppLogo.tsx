interface AppLogoProps {
  size?: number;
  className?: string;
}

export default function AppLogo({ size = 32, className }: AppLogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      aria-hidden
    >
      <rect width="32" height="32" rx="8" fill="#1677ff" />
      <path
        d="M8 22V10h4.5c2.2 0 3.5 1.1 3.5 2.7 0 1.1-.6 2-1.6 2.4 1.2.4 2 1.4 2 2.8 0 2-1.5 3.1-4 3.1H8zm2.2-7.8h2.1c1 0 1.5-.4 1.5-1.1 0-.7-.5-1.1-1.5-1.1h-2.1v2.2zm0 5.8h2.4c1.1 0 1.7-.5 1.7-1.3 0-.8-.6-1.2-1.7-1.2h-2.4v2.5z"
        fill="#fff"
      />
      <path
        d="M17.5 22l1.6-1.2c.8.9 1.7 1.4 2.8 1.4 1 0 1.6-.5 1.6-1.2 0-.8-.6-1-2.3-1.5-2-.6-3-1.5-3-3.2 0-1.8 1.5-3.1 3.7-3.1 1.5 0 2.7.5 3.7 1.5L23.5 15c-.7-.7-1.5-1.1-2.5-1.1-1 0-1.5.4-1.5 1 0 .7.5.9 2.1 1.4 2.1.6 3.1 1.5 3.1 3.3 0 2-1.6 3.4-4.1 3.4-1.6 0-2.9-.5-4.1-1.6z"
        fill="#fff"
      />
    </svg>
  );
}

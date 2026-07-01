import { createContext, useContext, type ReactNode } from "react";

interface QuickAddContextValue {
  openQuickAdd: () => void;
}

const QuickAddContext = createContext<QuickAddContextValue | null>(null);

export function QuickAddProvider({
  openQuickAdd,
  children,
}: {
  openQuickAdd: () => void;
  children: ReactNode;
}) {
  return <QuickAddContext.Provider value={{ openQuickAdd }}>{children}</QuickAddContext.Provider>;
}

export function useQuickAdd() {
  const ctx = useContext(QuickAddContext);
  if (!ctx) {
    throw new Error("useQuickAdd must be used within QuickAddProvider");
  }
  return ctx;
}

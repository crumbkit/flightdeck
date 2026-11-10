import { cn } from "cn";
import type { ReactNode } from "react";

type Props = {
  /** Whether the button shows the view on screen now. */
  on: boolean;
  onClick: () => void;
  children: ReactNode;
};

/** A button of the top bar or the usage view's tab bar, filled when its view is on screen. */
export function ViewButton({ on, onClick, children }: Props) {
  return (
    <button
      type="button"
      aria-current={on ? "page" : undefined}
      onClick={onClick}
      className={cn(
        "transition-tint inline-flex h-8 items-center rounded-md border px-2.5 text-xs font-medium",
        on
          ? "bg-accent border-accent text-accent-foreground"
          : "text-muted-foreground hover:bg-accent hover:text-accent-foreground border-transparent",
      )}
    >
      {children}
    </button>
  );
}

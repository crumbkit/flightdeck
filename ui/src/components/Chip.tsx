import { cn } from "cn";
import type { ReactNode } from "react";

type Props = {
  /** The one primary chip of a result or a failure; the others are quiet. */
  primary: boolean;
  /** Says more than the label, such as the sentence of a next step. */
  title?: string;
  onClick: () => void;
  children: ReactNode;
};

/**
 * The one shape of "do this next": a failure's recovery and alternatives
 * and a result's next steps. A chip fills the composer and runs nothing.
 */
export function Chip({ primary, title, onClick, children }: Props) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className={cn(
        "transition-tint min-h-8 rounded-full border px-3 py-1 font-mono text-xs active:brightness-90",
        primary
          ? "bg-primary text-primary-foreground border-primary hover:bg-primary-hover"
          : "bg-muted text-foreground hover:bg-accent hover:border-input",
      )}
    >
      {children}
    </button>
  );
}

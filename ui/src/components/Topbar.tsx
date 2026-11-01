import { Lock, Search } from "lucide-react";
import { ThemeControl } from "./ThemeControl";

type Props = { name: string; version: string; readOnly: boolean; onFind: (() => void) | null };

/** The bar across the top: the tool, the ⌘K search and the theme control. */
export function Topbar({ name, version, readOnly, onFind }: Props) {
  return (
    <header className="bg-card flex flex-wrap items-center gap-2 border-b px-3.5 py-2">
      <span className="font-semibold">flightdeck</span>
      <span className="text-muted-foreground font-mono">
        {name} <span className="text-faint-foreground text-xs">{version}</span>
      </span>
      {readOnly && (
        <span className="bg-muted inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs">
          <Lock aria-hidden="true" className="size-3" />
          read-only
        </span>
      )}
      <span className="ml-auto flex items-center gap-3">
        {onFind !== null && (
          <button
            type="button"
            onClick={onFind}
            className="bg-muted text-muted-foreground inline-flex items-center gap-2 rounded-md border px-2.5 py-1 text-xs"
          >
            <Search aria-hidden="true" className="size-3.5" />
            Find a command
            <kbd className="font-mono">⌘K</kbd>
          </button>
        )}
        <ThemeControl />
      </span>
    </header>
  );
}

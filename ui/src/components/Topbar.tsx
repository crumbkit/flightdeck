import { Lock, Search } from "lucide-react";
import type { ReactNode } from "react";
import { ThemeControl } from "./ThemeControl";
import { ViewButton } from "./ViewButton";

/** The view on screen. */
export type PageView = "commands" | "usage" | "environment";

type Props = {
  name: string;
  version: string;
  readOnly: boolean;
  onFind: (() => void) | null;
  /**
   * The view links, with the view on screen and whether the tool has a usage
   * view and an environment view, or null when it has neither, so the page
   * has one view.
   */
  views?: {
    current: PageView;
    usage: boolean;
    environment: boolean;
    onView: (view: PageView) => void;
  } | null;
  /** Opens the workspaces pages, or null when the bar offers no way there. */
  onWorkspaces?: (() => void) | null;
  /** What follows the tool's name, such as the directory control. */
  children?: ReactNode;
};

/**
 * The bar across the top: the tool, the view links, the way to the
 * workspaces, the ⌘K search and the theme control.
 */
export function Topbar({
  name,
  version,
  readOnly,
  onFind,
  views = null,
  onWorkspaces = null,
  children,
}: Props) {
  return (
    <header className="bg-card flex flex-wrap items-center gap-x-3 gap-y-2 border-b px-4 py-2">
      <span className="font-semibold tracking-tight">flightdeck</span>
      {name !== "" && (
        <span className="text-muted-foreground flex items-baseline gap-1.5 font-mono">
          <span aria-hidden="true" className="text-faint-foreground">
            /
          </span>
          {name} <span className="text-faint-foreground text-xs">{version}</span>
        </span>
      )}
      {readOnly && (
        <span className="bg-muted rounded-pill inline-flex items-center gap-1 border px-2 py-0.5 text-xs font-medium">
          <Lock aria-hidden="true" className="size-3" />
          read-only
        </span>
      )}
      {children}
      <span className="ml-auto flex flex-wrap items-center gap-3">
        {views !== null && (
          <nav aria-label="Views" className="flex items-center gap-1">
            <ViewButton on={views.current === "commands"} onClick={() => views.onView("commands")}>
              Commands
            </ViewButton>
            {views.usage && (
              <ViewButton on={views.current === "usage"} onClick={() => views.onView("usage")}>
                Usage
              </ViewButton>
            )}
            {views.environment && (
              <ViewButton
                on={views.current === "environment"}
                onClick={() => views.onView("environment")}
              >
                Environment
              </ViewButton>
            )}
          </nav>
        )}
        {onWorkspaces !== null && (
          <ViewButton on={false} onClick={onWorkspaces}>
            Workspaces
          </ViewButton>
        )}
        {onFind !== null && (
          <button
            type="button"
            onClick={onFind}
            className="bg-muted text-muted-foreground hover:bg-accent hover:text-foreground transition-tint active:bg-muted inline-flex h-8 items-center gap-2 rounded-md border px-2.5 text-xs whitespace-nowrap"
          >
            <Search aria-hidden="true" className="size-3.5" />
            Find a command
            <kbd className="hidden font-mono md:inline">⌘K</kbd>
          </button>
        )}
        <ThemeControl />
      </span>
    </header>
  );
}

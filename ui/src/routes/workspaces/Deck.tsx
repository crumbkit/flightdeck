import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import type { ReactNode } from "react";
import { workspacesQuery } from "@/api/workspaces";
import { hrefOf, type Place } from "@/lib/place";
import { Label } from "./parts";
import { Workspace } from "./Workspace";
import { Workspaces } from "./Workspaces";

/**
 * One link of the rail, filled when it names what is on screen, with an
 * optional count. `current` says what the filled link names: the page, or
 * the chosen item of a set, such as the target.
 */
function RailLink({
  href,
  on,
  current = "page",
  count,
  children,
}: {
  href: string;
  on: boolean;
  current?: "page" | "true";
  count?: number | undefined;
  children: ReactNode;
}) {
  return (
    <a
      href={href}
      aria-current={on ? current : undefined}
      className={cn(
        "transition-tint flex items-center justify-between rounded-md px-2 py-2.5",
        on ? "bg-accent text-accent-foreground font-medium" : "text-foreground hover:bg-muted",
      )}
    >
      <span>{children}</span>
      {count !== undefined && <span className="text-muted-foreground text-xs">{count}</span>}
    </a>
  );
}

type Props = {
  /** The place that the address names. The start shows the workspaces list. */
  place: Place;
  /** Whether flightdeck holds a tool, so the rail links to its commands. */
  tool: boolean;
  /** Called when the server refuses the page's token. */
  onSignedOut: () => void;
};

/**
 * The workspaces pages: the rail, with the target and the views, beside
 * the Workspaces page or the One workspace page. The rail's counts come
 * from the Workspaces page's last answer, so the One workspace page does
 * not ask mxv about every workspace.
 */
export default function Deck({ place, tool, onSignedOut }: Props) {
  const list = useQuery({ ...workspacesQuery(), enabled: false });
  const count = list.data?.summary.workspaces;
  const listHref = hrefOf({ view: "workspaces" });
  return (
    <div className="flex h-full flex-col md:flex-row">
      <div className="bg-rail flex shrink-0 flex-col gap-4 border-b px-3 py-4 md:w-64 md:border-r md:border-b-0">
        <span className="px-2 font-semibold tracking-tight">flightdeck</span>
        <nav aria-label="Target">
          <Label>
            <span className="px-2">Target</span>
          </Label>
          <RailLink href={listHref} on current="true" count={count}>
            local · this Mac
          </RailLink>
        </nav>
        <nav aria-label="Views">
          <Label>
            <span className="px-2">View</span>
          </Label>
          <RailLink href={listHref} on count={count}>
            Workspaces
          </RailLink>
          {tool && (
            <RailLink href={hrefOf({ view: "start" })} on={false}>
              Commands
            </RailLink>
          )}
        </nav>
      </div>
      <main className="min-w-0 flex-1 overflow-y-auto p-5">
        {place.view === "workspace" ? (
          <Workspace key={place.name} name={place.name} onSignedOut={onSignedOut} />
        ) : (
          <Workspaces onSignedOut={onSignedOut} />
        )}
      </main>
    </div>
  );
}

import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { lazy, type ReactNode, Suspense, useState } from "react";
import { workflowsQuery } from "@/api/workflows";
import { workspacesQuery } from "@/api/workspaces";
import { hrefOf, type Place } from "@/lib/place";
import { Loading } from "@/routes/Loading";
import { usePhone } from "@/routes/workflows/phone";
import "./deck.css";
import { Label } from "./parts";
import { Workspace } from "./Workspace";
import { Workspaces } from "./Workspaces";

// The Workflows page loads in its own file the first time it shows, so the
// workspaces pages stay within their size budget.
const Workflows = lazy(() => import("@/routes/workflows/Workflows"));

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

/** The page of `place`: the Workflows page, one workspace or the workspaces list. */
function Page({ place, onSignedOut }: { place: Place; onSignedOut: () => void }) {
  if (place.view === "workflows") {
    return (
      <Suspense fallback={<Loading />}>
        <Workflows slug={place.slug} selected={place.selected} onSignedOut={onSignedOut} />
      </Suspense>
    );
  }
  if (place.view === "workspace") {
    return <Workspace key={place.name} name={place.name} onSignedOut={onSignedOut} />;
  }
  return <Workspaces onSignedOut={onSignedOut} />;
}

type Props = {
  /** The place that the address names. The start shows the workspaces list. */
  place: Place;
  /** Whether flightdeck holds a tool, so the rail links to its commands. */
  tool: boolean;
  /** Whether the board program answered, so the rail links to the Workflows page. */
  board: boolean;
  /** Called when the server refuses the page's token. */
  onSignedOut: () => void;
};

/**
 * The workspaces and Workflows pages: the rail, with the target and the
 * views, beside the Workspaces page, the One workspace page or the
 * Workflows page. The rail's workspace counts come from the Workspaces
 * page's last answer, so the One workspace page does not ask mxv about
 * every workspace. The Workflows entry shows only when the board program
 * answered, with the number of graphs. On a phone the Workflows page keeps
 * the rail behind a Views button.
 */
export default function Deck({ place, tool, board, onSignedOut }: Props) {
  const list = useQuery({ ...workspacesQuery(), enabled: false });
  const graphs = useQuery({ ...workflowsQuery(), enabled: board });
  const count = list.data?.summary.workspaces;
  const listHref = hrefOf({ view: "workspaces" });
  const workflows = place.view === "workflows";
  const phone = usePhone();
  const [views, setViews] = useState(false);
  const folded = workflows && phone;
  return (
    <div className="deck">
      {folded && (
        <div className="deck-ptop">
          <button
            type="button"
            className="deck-views"
            aria-expanded={views}
            aria-controls="deck-rail"
            onClick={() => setViews(!views)}
          >
            Views
          </button>
          <span className="text-muted-foreground text-xs">flightdeck</span>
        </div>
      )}
      <div id="deck-rail" className="bg-rail deck-rail" hidden={folded && !views}>
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
          <RailLink href={listHref} on={!workflows} count={count}>
            Workspaces
          </RailLink>
          {board && (
            <RailLink
              href={hrefOf({ view: "workflows", slug: null, selected: null })}
              on={workflows}
              count={graphs.data?.summary.graphs}
            >
              Workflows
            </RailLink>
          )}
          {tool && (
            <RailLink href={hrefOf({ view: "start" })} on={false}>
              Commands
            </RailLink>
          )}
        </nav>
      </div>
      <main
        className={cn(
          "deck-main",
          (place.view === "workspace" || workflows) && "deck-main-workspace",
          folded && "deck-main-phone",
        )}
      >
        <Page place={place} onSignedOut={onSignedOut} />
      </main>
    </div>
  );
}

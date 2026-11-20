import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { isSignedOut } from "@/api/failure";
import { type WorkspaceRow, workspacesQuery } from "@/api/workspaces";
import { Failure } from "@/components/Failure";
import { hrefOf } from "@/lib/place";
import { addressesOf, dotsOf, unitsLine } from "@/lib/workspaces";
import { Dot, Pill, RoutesNote } from "./parts";

/** Returns the footer's word for whether mxv keeps the workspace from reclaiming. */
function pinnedWord(pinned: boolean | null): string {
  if (pinned === null) {
    return "unknown";
  }
  return pinned ? "yes" : "no";
}

/**
 * One workspace's card, which links to its page: its name, its branch,
 * its checkouts, its units, its routes or host ports, and whether it is
 * pinned, or unknown when its `mxv status` failed. The card is the link, so its addresses are words, not links.
 */
function Card({ row }: { row: WorkspaceRow }) {
  const addresses = addressesOf(row);
  const dots = dotsOf(row);
  return (
    <a href={hrefOf({ view: "workspace", name: row.name })} className="deck-card transition-tint">
      <span className="font-mono font-medium wrap-anywhere">{row.name}</span>
      <dl className="deck-kv">
        <dt className="text-muted-foreground">Branch</dt>
        <dd className="font-mono wrap-anywhere">{row.branch ?? "unknown"}</dd>
        <dt className="text-muted-foreground">Repos</dt>
        <dd className="font-mono wrap-anywhere">{row.checkouts.join(", ")}</dd>
        <dt className="text-muted-foreground">Units</dt>
        <dd className="deck-dots">
          {dots.map((tone, index) => (
            // A unit's dot keeps its place: mxv lists the units in one order.
            // biome-ignore lint/suspicious/noArrayIndexKey: the dots never reorder
            <Dot key={index} tone={tone} />
          ))}
          <span>{unitsLine(row)}</span>
        </dd>
        <dt className="text-muted-foreground">Routes</dt>
        <dd className="flex flex-col font-mono wrap-anywhere">
          {addresses.length === 0 ? (
            <span className="font-sans">none</span>
          ) : (
            addresses.map((address) => <span key={address}>{address}</span>)
          )}
        </dd>
      </dl>
      {row.problem !== null && <p className="text-destructive m-0">{row.problem}</p>}
      <span className="deck-foot text-muted-foreground text-xs">
        Pinned: {pinnedWord(row.pinned)}
      </span>
    </a>
  );
}

/** What the page shows when mxv lists no workspace: how to make one. */
function Empty() {
  return (
    <div className="deck-card max-w-xl">
      <p className="m-0 font-medium">mxv lists no workspace.</p>
      <p className="text-muted-foreground m-0">
        Create one in a terminal with{" "}
        <code className="font-mono">mxv new &lt;name&gt; --repo &lt;path&gt;</code>, and its card
        appears here.
      </p>
    </div>
  );
}

type Props = {
  /** Called when the server refuses the page's token. */
  onSignedOut: () => void;
};

/**
 * The Workspaces page: one card per workspace that mxv lists on this
 * machine, with how many run and how many are stopped. It asks the server
 * again every 10 seconds while the tab is visible.
 */
export function Workspaces({ onSignedOut }: Props) {
  const list = useQuery(workspacesQuery());
  const refused = isSignedOut(list.error);

  useEffect(() => {
    if (refused) {
      onSignedOut();
    }
  }, [refused, onSignedOut]);

  return (
    <div className="deck-list">
      <div className="deck-header">
        <div>
          <h1 className="m-0 text-xl font-semibold">
            Workspaces on {list.data?.summary.target ?? "local"}
          </h1>
          <p className="text-muted-foreground m-0">Each card is one mxv workspace.</p>
        </div>
        {list.data !== undefined && (
          <div className="flex gap-2">
            <Pill on={list.data.summary.running > 0}>{list.data.summary.running} running</Pill>
            <Pill>{list.data.summary.stopped} stopped</Pill>
          </div>
        )}
      </div>
      {list.isPending && (
        <p aria-busy="true" className="text-muted-foreground m-0">
          Asking mxv for the workspaces…
        </p>
      )}
      {list.isError && !refused && (
        <Failure error={list.error} onRetry={() => void list.refetch()} />
      )}
      {list.data !== undefined && (
        <>
          <RoutesNote routes={list.data.summary.routes} then="Cards show host ports." />
          {list.data.data.length === 0 ? (
            <Empty />
          ) : (
            <div className="deck-grid">
              {list.data.data.map((row) => (
                <Card key={row.name} row={row} />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

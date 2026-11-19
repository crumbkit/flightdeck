import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { ApiError } from "@/api/client";
import { isSignedOut } from "@/api/failure";
import { type CheckoutState, type WorkspaceShow, workspaceQuery } from "@/api/workspaces";
import { Failure } from "@/components/Failure";
import { hrefOf } from "@/lib/place";
import { unitTone, unitWord } from "@/lib/workspaces";
import { Dot, Label, RoutesNote } from "./parts";

const list = hrefOf({ view: "workspaces" });

/** Returns true when the server says mxv knows no workspace by the name asked for. */
function isNotFound(error: unknown): boolean {
  return (
    error instanceof ApiError && error.failure.kind === "status" && error.failure.status === 404
  );
}

/** Returns a checkout's words for its uncommitted changes, or null when mxv cannot tell. */
function changesWords(changes: number | null): string | null {
  if (changes === null) {
    return null;
  }
  if (changes === 0) {
    return "no uncommitted changes";
  }
  return changes === 1 ? "1 uncommitted change" : `${changes} uncommitted changes`;
}

/** One checkout: its folder, then its branch, its short commit and its uncommitted changes. */
function Checkout({ checkout }: { checkout: CheckoutState }) {
  const changes = changesWords(checkout.changes);
  return (
    <div className="flex flex-col gap-1">
      <span className="font-mono font-medium">{checkout.folder}</span>
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-muted-foreground">Branch</span>
        <span className="font-mono">{checkout.branch ?? "unknown"}</span>
        {checkout.commit !== null && (
          <span className="text-muted-foreground font-mono">{checkout.commit.slice(0, 7)}</span>
        )}
        {changes !== null && <span className="text-muted-foreground">{changes}</span>}
      </div>
      {checkout.problem !== null && <p className="text-destructive m-0">{checkout.problem}</p>}
    </div>
  );
}

/** The units table: each unit's name, state, ports and route links. */
function Units({ workspace }: { workspace: WorkspaceShow }) {
  if (workspace.units.length === 0) {
    return workspace.manifest_error !== null ? (
      <p className="text-destructive m-0">{workspace.manifest_error}</p>
    ) : (
      <p className="text-muted-foreground m-0">The workspace's manifest declares no unit.</p>
    );
  }
  return (
    <table aria-label="Units" className="deck-units">
      <colgroup>
        <col />
        <col />
        <col />
        <col />
      </colgroup>
      <thead>
        <tr className="text-muted-foreground text-xs">
          <th>Unit</th>
          <th>State</th>
          <th>Port</th>
          <th>Open</th>
        </tr>
      </thead>
      <tbody>
        {workspace.units.map((unit) => (
          <tr key={unit.name}>
            <td className="font-mono">{unit.name}</td>
            <td>
              <span className="inline-flex items-center gap-1.5">
                <Dot tone={unitTone(unit)} />
                {unitWord(unit)}
              </span>
            </td>
            <td className="font-mono wrap-anywhere">
              {unit.ports.map((port) => (
                <div key={port}>{port}</div>
              ))}
            </td>
            <td>
              {unit.routes.map((url, index) => (
                <span key={url}>
                  {index > 0 && " · "}
                  <a href={url}>route</a>
                </span>
              ))}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

type Props = {
  /** The workspace's name, as the page's address gives it. */
  name: string;
  /** Called when the server refuses the page's token. */
  onSignedOut: () => void;
};

/**
 * The One workspace page: its checkouts and its units, from `mxv status`.
 * It asks the server again every 5 seconds while the tab is visible.
 */
export function Workspace({ name, onSignedOut }: Props) {
  const shown = useQuery(workspaceQuery(name));
  const refused = isSignedOut(shown.error);

  useEffect(() => {
    if (refused) {
      onSignedOut();
    }
  }, [refused, onSignedOut]);

  return (
    <div className="flex flex-col gap-4">
      <div>
        <nav aria-label="Breadcrumb" className="deck-crumbs text-muted-foreground">
          <a href={list}>local</a>
          <span aria-hidden="true">/</span>
          <span className="font-mono">{name}</span>
        </nav>
        <h1 className="m-0 font-mono text-xl font-semibold wrap-anywhere">{name}</h1>
      </div>
      {shown.isPending && (
        <p aria-busy="true" className="text-muted-foreground m-0">
          Asking mxv for the workspace…
        </p>
      )}
      {isNotFound(shown.error) && (
        <div className="deck-card max-w-xl items-start">
          <p className="m-0">No workspace is named {name}.</p>
          <a href={list} className="text-primary underline underline-offset-2">
            Back to the workspaces
          </a>
        </div>
      )}
      {shown.isError && !refused && !isNotFound(shown.error) && (
        <Failure error={shown.error} onRetry={() => void shown.refetch()} />
      )}
      {shown.data !== undefined && (
        <div className="deck-cols">
          <section aria-label="Checkouts" className="deck-card">
            <Label>Checkouts · from mxv status</Label>
            {shown.data.checkouts.map((checkout) => (
              <Checkout key={checkout.folder} checkout={checkout} />
            ))}
          </section>
          <section aria-label="Units" className="deck-card">
            <Label>Units · from mxv status</Label>
            <Units workspace={shown.data} />
            <RoutesNote routes={shown.data.routes} then="The table shows no route link." />
          </section>
        </div>
      )}
    </div>
  );
}

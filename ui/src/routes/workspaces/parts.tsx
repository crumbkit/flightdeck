import { cn } from "cn";
import type { ReactNode } from "react";
import type { Routes } from "@/api/workspaces";
import type { Tone } from "@/lib/workspaces";

/** The fill of each tone's dot: green for running, red for exited, hollow otherwise. */
const fills: Record<Tone, string> = {
  on: "bg-read-only",
  bad: "bg-destructive",
  off: "bg-card border-input border",
};

/** A unit's state as a dot, always beside the state's words. */
export function Dot({ tone }: { tone: Tone }) {
  return <span aria-hidden="true" className={cn("inline-flex size-2 rounded-full", fills[tone])} />;
}

/** The small capitals label of a card or a rail group. */
export function Label({ children }: { children: ReactNode }) {
  return (
    <div className="text-muted-foreground text-2xs mb-1.5 font-medium tracking-wide uppercase">
      {children}
    </div>
  );
}

/** A count or a fact in a pill, green when `on`. */
export function Pill({ on = false, children }: { on?: boolean; children: ReactNode }) {
  return (
    <span
      className={cn(
        "rounded-pill inline-flex items-center border px-2 py-0.5 text-xs whitespace-nowrap",
        on ? "bg-read-only-soft text-read-only border-transparent" : "bg-card",
      )}
    >
      {children}
    </span>
  );
}

/** Says why routes are missing when `mxv proxy status` did not answer, and nothing otherwise. */
export function RoutesNote({ routes, then }: { routes: Routes; then: string }) {
  if (routes.state === "available") {
    return null;
  }
  return (
    <p className="text-muted-foreground m-0">
      Routes are unavailable: {routes.reason}. {then}
    </p>
  );
}

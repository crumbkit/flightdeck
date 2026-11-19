import type { UnitDetail, WorkspaceRow } from "@/api/workspaces";

/** How a unit's dot draws: running, failed, or anything else. */
export type Tone = "on" | "bad" | "off";

/** Returns the tone of a unit in `state`: running is on, exited is failed. */
function toneOf(state: string): Tone {
  if (state === "running") {
    return "on";
  }
  return state === "exited" ? "bad" : "off";
}

/**
 * Returns a card's Units line: how many units run out of those declared,
 * "stopped" when none of them runs, "none declared", or "units unknown"
 * with mxv's code when its status of the workspace failed.
 */
export function unitsLine(row: WorkspaceRow): string {
  if (row.running === null || row.declared === null) {
    const code = row.status_error?.code ?? null;
    return code === null ? "units unknown" : `units unknown · ${code}`;
  }
  if (row.declared === 0) {
    return "none declared";
  }
  return row.running === 0 ? "stopped" : `${row.running} of ${row.declared} running`;
}

/** Returns one dot per unit of a card, in mxv's order, or none when the units are unknown. */
export function dotsOf(row: WorkspaceRow): Tone[] {
  return (row.units ?? []).map((unit) => toneOf(unit.state));
}

/** An address on a card: its words, and the URL it links to, or null for a host port. */
export type Address = { text: string; href: string | null };

/**
 * Returns a card's Routes: each route that the proxy serves, by its host
 * name, or else each host port of the workspace on 127.0.0.1 in the order
 * of the ports' names.
 */
export function addressesOf(row: WorkspaceRow): Address[] {
  if (row.routes.length > 0) {
    return row.routes.map((route) => ({ text: hostOf(route.url), href: route.url }));
  }
  return Object.keys(row.ports)
    .sort()
    .map((name) => ({ text: `127.0.0.1:${row.ports[name]}`, href: null }));
}

/** Returns the host of `url`, or `url` itself when it does not parse. */
function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

/**
 * Returns the State column's words for a unit: "ready" for a running unit
 * that passed its readiness check, its exit code or signal for an exited
 * one, and mxv's own word otherwise.
 */
export function unitWord(unit: UnitDetail): string {
  if (unit.state === "running") {
    return unit.ready === false ? "running, not ready" : "ready";
  }
  if (unit.state === "exited") {
    if (unit.code !== null) {
      return `exited with ${unit.code}`;
    }
    if (unit.signal !== null) {
      return `exited on signal ${unit.signal}`;
    }
  }
  return unit.state;
}

/** Returns the tone of a unit's dot in the units table. */
export function unitTone(unit: UnitDetail): Tone {
  return toneOf(unit.state);
}

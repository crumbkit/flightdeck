// Where a repository differs from the imposed rule set, as the Rules page,
// a control's page and the groups page word it. Drift sits beside a cell's
// state and never changes it. Only lacking or overriding an imposed rule, or
// reading another set or an older commit of it, counts as drift; the rules a
// repository adds for its own app are listed apart.
import type { RepositoryColumn } from "./document.ts";

/** Whether a repository matches the imposed set, drifts from it, or cannot be told for want of a report. */
export type DriftState = "matches" | "drifts" | "unknown";

/** Returns the word of a drift state. */
export function driftWord(state: DriftState): string {
  switch (state) {
    case "matches":
      return "matches";
    case "drifts":
      return "drifts";
    case "unknown":
      return "unknown";
  }
}

/**
 * Returns each way a repository drifts, such as "Overrides /rules/x:
 * requirement" or "Lacks /rules/y". "Lacks" names an imposed rule that
 * reaches none of the repository's files, which the report lists as not
 * reached.
 */
export function driftItems(column: RepositoryColumn): string[] {
  const drift = column.drift;
  if (drift === null) return [];
  return [
    ...drift.differs.map((each) => `Overrides ${each.id}: ${each.fields.join(", ")}`),
    ...drift.not_reached.map((rule) => `Lacks ${rule}`),
    ...(drift.other_set ? [`Reads another rule set: ${column.rules_from ?? "unknown"}`] : []),
    ...(drift.behind === true ? ["Reads an older commit of the imposed set"] : []),
  ];
}

/** Returns whether a repository matches the imposed set. */
export function driftState(column: RepositoryColumn): DriftState {
  if (column.drift === null) return "unknown";
  return driftItems(column).length === 0 ? "matches" : "drifts";
}

/** Whether a repository drifts on the imposed rule `rule`: it overrides it or lacks it. */
export function driftsOn(column: RepositoryColumn, rule: string): boolean {
  const drift = column.drift;
  if (drift === null) return false;
  return drift.differs.some((each) => each.id === rule) || drift.not_reached.includes(rule);
}

/** Returns the rules a repository adds for its own app, or null when it sent no report. */
export function addedRules(column: RepositoryColumn): readonly string[] | null {
  return column.drift?.extra ?? null;
}

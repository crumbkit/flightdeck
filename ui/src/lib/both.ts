import type { Day, Tool } from "@/api/model";
import type { WeightReply } from "@/api/weight";
import { grouped, withPeriod } from "./cost";
import { budgets, weightPaths } from "./weight";

/**
 * The path weight from which a command is heavy: half of crumbkit's
 * default path budget.
 */
export const heavyBytes = budgets.path / 2;

/** A corner of the both table: heavy or light, and run often or never. */
export type Corner = "heavy_often" | "light_never" | "heavy_never" | "light_often";

/** The corners in the order their chips show. */
export const corners: Corner[] = ["heavy_often", "light_never", "heavy_never", "light_often"];

/** Each corner's chip label. */
export const cornerLabels: Record<Corner, string> = {
  heavy_often: "Heavy and often",
  light_never: "Light and never",
  heavy_never: "Heavy and never",
  light_often: "Light and often",
};

/** What each corner suggests, in the line under the chips. */
export const cornerHints: Record<Corner, string> = {
  heavy_often: "trim the output or the help pages, or give the command a shorter path.",
  light_never: "question whether the command is needed.",
  heavy_never: "the cheapest cut.",
  light_often: "nothing to do.",
};

/**
 * One row of the both table: a command, its path weight and whether a page
 * of the path was not measured, its runs and the bytes they read, both
 * null when the figures do not count the command, and its corner, or null
 * when it is in none.
 */
export type BothRow = {
  command: string;
  bytes: number;
  partial: boolean;
  runs: number | null;
  read: number | null;
  corner: Corner | null;
};

/**
 * Returns whether a command can take a corner: counted by the figures and
 * not one that crumbkit adds to every tool, as the server marks it.
 */
function cornered(command: Tool["commands"][number]): boolean {
  return command.counted && !command.built_in;
}

/**
 * Returns the runs from which a command runs often: the median runs of the
 * commands that ran, among those that can take a corner, or null when none
 * ran.
 */
export function oftenRuns(tool: Tool): number | null {
  const runs = tool.commands
    .filter(cornered)
    .map((command) => command.runs)
    .filter((count) => count > 0)
    .sort((a, b) => a - b);
  if (runs.length === 0) {
    return null;
  }
  const middle = Math.floor(runs.length / 2);
  const upper = runs[middle] ?? 0;
  return runs.length % 2 === 1 ? upper : ((runs[middle - 1] ?? 0) + upper) / 2;
}

/** Returns the corner of a command with `bytes` of path weight and `runs`, or null. */
function cornerOf(bytes: number, runs: number, often: number | null): Corner | null {
  const heavy = bytes >= heavyBytes;
  if (runs === 0) {
    return heavy ? "heavy_never" : "light_never";
  }
  if (often !== null && runs >= often) {
    return heavy ? "heavy_often" : "light_often";
  }
  return null;
}

/**
 * Returns a row for each of the tool's commands, from the weight tab's
 * measurement and the figures, the most read first, with the commands the
 * figures do not count last. Rows that read the same, such as the commands
 * that never ran, list the heaviest first, so the biggest cut leads, and
 * then in name order. A command takes a
 * corner only when the figures count it, crumbkit did not add it, and every
 * page of its path was measured.
 */
export function bothRows(tool: Tool, reply: WeightReply): BothRow[] {
  const often = oftenRuns(tool);
  const paths = weightPaths(tool, reply);
  const rows = tool.commands.map((command, index): BothRow => {
    const path = paths[index];
    const bytes = path?.total ?? 0;
    const partial = path?.partial ?? true;
    const runs = command.counted ? command.runs : null;
    return {
      command: command.path.join(" "),
      bytes,
      partial,
      runs,
      read: runs === null ? null : bytes * runs,
      corner: cornered(command) && !partial ? cornerOf(bytes, command.runs, often) : null,
    };
  });
  return rows.sort(
    (a, b) =>
      (b.read ?? -1) - (a.read ?? -1) || b.bytes - a.bytes || a.command.localeCompare(b.command),
  );
}

/** Returns how many rows each corner holds. */
export function cornerCounts(rows: BothRow[]): Record<Corner, number> {
  const counts = { heavy_often: 0, light_never: 0, heavy_never: 0, light_often: 0 };
  for (const row of rows) {
    if (row.corner !== null) {
      counts[row.corner] += 1;
    }
  }
  return counts;
}

/**
 * Returns the parts of the both tab's summary: what the table multiplies
 * with the window, then the thresholds of heavy and often, which a phone
 * leaves out.
 */
export function bothSummary(tool: Tool, today: Day): [string, string, string] {
  const often = oftenRuns(tool);
  return [
    withPeriod("Path weight times runs", tool.figures_since, today),
    `heavy is at least ${grouped(heavyBytes)} B, half the default path budget`,
    often === null
      ? "no command ran, so none runs often"
      : `often is at least ${grouped(often)} runs, the median of commands that ran`,
  ];
}

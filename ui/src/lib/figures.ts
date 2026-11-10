import type { Command, Day, Figures } from "@/api/model";
import { withPeriod } from "./cost";
import { counted } from "./words";

/**
 * One row of the figures table: a command, its figures or null when it
 * ran no time in the window, and whether the figures count its runs at all.
 */
export type FigureRow = { command: string; figures: Figures | null; counted: boolean };

/** A column the figures table sorts by. */
export type FigureKey =
  | "command"
  | "runs"
  | "failed"
  | "refused"
  | "help_before"
  | "p50_ms"
  | "p95_ms"
  | "p50_out_bytes"
  | "p95_out_bytes";

export type Direction = "ascending" | "descending";

/**
 * Returns a row for each of `commands`, in their order. A figures row with
 * no runs gives the command no figures, as a command the figures hold no
 * row for has none, so the table shows the same dashes for both.
 */
export function figureRows(commands: Command[]): FigureRow[] {
  return commands.map((command) => {
    const figures = command.cost?.figures ?? null;
    return {
      command: command.path.join(" "),
      figures: figures !== null && figures.runs > 0 ? figures : null,
      counted: command.counted,
    };
  });
}

/**
 * Returns the figure of `row` that `key` names, or null when the row has
 * none. A counted command with no figures ran no time, so its runs are 0;
 * a command the figures do not count has no figure at all.
 */
export function figureOf(row: FigureRow, key: Exclude<FigureKey, "command">): number | null {
  if (!row.counted) {
    return null;
  }
  if (row.figures === null) {
    return key === "runs" ? 0 : null;
  }
  return row.figures[key];
}

/**
 * Returns `rows` sorted by `key` in `direction`. A command the figures do
 * not count goes last, and then a row with no figure for the key, in
 * either direction. Ties go in name order, so the table keeps one order
 * for the same figures.
 */
export function sortedRows(rows: FigureRow[], key: FigureKey, direction: Direction): FigureRow[] {
  const sign = direction === "ascending" ? 1 : -1;
  const byName = (a: FigureRow, b: FigureRow) => a.command.localeCompare(b.command);
  return [...rows].sort((a, b) => {
    if (a.counted !== b.counted) {
      return Number(b.counted) - Number(a.counted);
    }
    if (key === "command") {
      return sign * byName(a, b);
    }
    const left = figureOf(a, key);
    const right = figureOf(b, key);
    if (left === null || right === null) {
      return Number(left === null) - Number(right === null) || byName(a, b);
    }
    return sign * (left - right) || byName(a, b);
  });
}

/**
 * Returns the figures tab's summary: the runs, the commands, and the days
 * the figures cover when they name the first, such as "2,727 runs of 19
 * commands, 3 Nov to 10 Nov".
 */
export function figuresSummary(rows: FigureRow[], since: Day | null, today: Day): string {
  const runs = rows.reduce((sum, row) => sum + (figureOf(row, "runs") ?? 0), 0);
  return withPeriod(`${counted(runs, "run")} of ${counted(rows.length, "command")}`, since, today);
}

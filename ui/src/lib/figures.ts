import type { Command, Day, Figures } from "@/api/model";
import { dayWords } from "./cost";
import { counted } from "./words";

/** One row of the figures table: a command, and its figures or null when it has none. */
export type FigureRow = { command: string; figures: Figures | null };

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

/** Returns a row for each of `commands`, in their order. */
export function figureRows(commands: Command[]): FigureRow[] {
  return commands.map((command) => ({
    command: command.path.join(" "),
    figures: command.cost?.figures ?? null,
  }));
}

/** Returns the figure of `row` that `key` names, or null when the row has none. */
function figureOf(row: FigureRow, key: Exclude<FigureKey, "command">): number | null {
  if (row.figures === null) {
    // A command the figures hold no row for never ran in the window.
    return key === "runs" ? 0 : null;
  }
  return row.figures[key];
}

/**
 * Returns `rows` sorted by `key` in `direction`. A row with no figure for
 * the key goes last in either direction, and ties go in name order, so the
 * table keeps one order for the same figures.
 */
export function sortedRows(rows: FigureRow[], key: FigureKey, direction: Direction): FigureRow[] {
  const sign = direction === "ascending" ? 1 : -1;
  return [...rows].sort((a, b) => {
    if (key === "command") {
      return sign * a.command.localeCompare(b.command);
    }
    const left = figureOf(a, key);
    const right = figureOf(b, key);
    if (left === null || right === null) {
      return Number(left === null) - Number(right === null) || a.command.localeCompare(b.command);
    }
    return sign * (left - right) || a.command.localeCompare(b.command);
  });
}

/**
 * Returns the figures tab's summary: the runs, the commands, and the days
 * the figures cover when they name the first, such as "2,727 runs of 19
 * commands, 3 Nov to 10 Nov".
 */
export function figuresSummary(rows: FigureRow[], since: Day | null, today: Day): string {
  const runs = rows.reduce((sum, row) => sum + (row.figures?.runs ?? 0), 0);
  const count = `${counted(runs, "run")} of ${counted(rows.length, "command")}`;
  return since === null ? count : `${count}, ${dayWords(since)} to ${dayWords(today)}`;
}

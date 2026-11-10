import { describe, expect, it } from "vitest";
import type { Figures } from "@/api/model";
import { command as commandOf, shelf } from "../../tests/fixtures/shelf";
import { type FigureRow, figureRows, figuresSummary, sortedRows } from "./figures";

/** Returns figures with `runs` runs and the rest as `change` gives them. */
function figures(runs: number, change: Partial<Figures> = {}): Figures {
  return {
    runs,
    help_pages: 0,
    help_before: 0,
    refused: 0,
    failed: 0,
    p50_ms: 10,
    p95_ms: 20,
    p50_out_bytes: 100,
    p95_out_bytes: 200,
    ...change,
  };
}

/** Returns a counted row of `command` with `figures`, or with none. */
function row(command: string, of: Figures | null): FigureRow {
  return { command, figures: of, counted: true };
}

const today = { year: 2026, month: 11, day: 10 };

describe("figureRows", () => {
  it("gives every command a row, with no figures for a command that has no figures row or a row of no runs", () => {
    const costed = (path: string, of: Figures) => ({
      ...commandOf(shelf, path),
      cost: { figures: of, findings: null },
    });

    const rows = figureRows([
      costed("books list", figures(3)),
      costed("books summarize", figures(0)),
      commandOf(shelf, "books show"),
      commandOf(shelf, "self usage"),
    ]);

    expect(rows).toEqual([
      row("books list", figures(3)),
      row("books summarize", null),
      row("books show", null),
      { command: "self usage", figures: null, counted: false },
    ]);
  });
});

describe("sortedRows", () => {
  const rows = [
    row("books show", figures(5, { failed: 2, p95_ms: 90 })),
    row("books add", null),
    row("books list", figures(9, { failed: 0, p95_ms: null })),
    row("loans list", figures(5, { failed: 1, p95_ms: 40 })),
    { command: "about", figures: null, counted: false },
  ];
  const order = (key: Parameters<typeof sortedRows>[1], direction: "ascending" | "descending") =>
    sortedRows(rows, key, direction).map((each) => each.command);

  it("sorts by runs, most first, with ties in name order", () => {
    expect(order("runs", "descending")).toEqual([
      "books list",
      "books show",
      "loans list",
      "books add",
      "about",
    ]);
  });

  it("puts a command the figures do not count last, then one with no figure for the key, in either direction", () => {
    expect(order("p95_ms", "descending")).toEqual([
      "books show",
      "loans list",
      "books add",
      "books list",
      "about",
    ]);
    expect(order("p95_ms", "ascending")).toEqual([
      "loans list",
      "books show",
      "books add",
      "books list",
      "about",
    ]);
  });

  it("sorts by command name in either direction", () => {
    expect(order("command", "ascending")).toEqual([
      "books add",
      "books list",
      "books show",
      "loans list",
      "about",
    ]);
    expect(order("command", "descending")).toEqual([
      "loans list",
      "books show",
      "books list",
      "books add",
      "about",
    ]);
  });
});

describe("figuresSummary", () => {
  it("counts the runs and commands, and the days when the figures name their first", () => {
    const rows = [
      row("books list", figures(2_700)),
      row("books show", figures(27)),
      row("books add", null),
      { command: "self usage", figures: figures(9), counted: false },
    ];

    expect(figuresSummary(rows, { year: 2026, month: 11, day: 3 }, today)).toBe(
      "2,727 runs of 4 commands, 3 Nov to 10 Nov",
    );
    expect(figuresSummary(rows, null, today)).toBe("2,727 runs of 4 commands");
  });
});

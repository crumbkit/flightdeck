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

/** Returns a row of `command` with `figures`, or with none. */
function row(command: string, of: Figures | null): FigureRow {
  return { command, figures: of };
}

const today = { year: 2026, month: 11, day: 10 };

describe("figureRows", () => {
  it("gives every command a row, with null figures for a command the figures hold no row for", () => {
    const list = {
      ...commandOf(shelf, "books list"),
      cost: { figures: figures(3), findings: null },
    };

    const rows = figureRows([list, commandOf(shelf, "books show")]);

    expect(rows).toEqual([row("books list", figures(3)), row("books show", null)]);
  });
});

describe("sortedRows", () => {
  const rows = [
    row("books show", figures(5, { failed: 2, p95_ms: 90 })),
    row("books add", null),
    row("books list", figures(9, { failed: 0, p95_ms: null })),
    row("loans list", figures(5, { failed: 1, p95_ms: 40 })),
  ];
  const order = (key: Parameters<typeof sortedRows>[1], direction: "ascending" | "descending") =>
    sortedRows(rows, key, direction).map((each) => each.command);

  it("sorts by runs, most first, with ties in name order", () => {
    expect(order("runs", "descending")).toEqual([
      "books list",
      "books show",
      "loans list",
      "books add",
    ]);
  });

  it("puts a command with no figure for the key last in either direction", () => {
    expect(order("p95_ms", "descending")).toEqual([
      "books show",
      "loans list",
      "books add",
      "books list",
    ]);
    expect(order("p95_ms", "ascending")).toEqual([
      "loans list",
      "books show",
      "books add",
      "books list",
    ]);
  });

  it("sorts by command name in either direction", () => {
    expect(order("command", "ascending")).toEqual([
      "books add",
      "books list",
      "books show",
      "loans list",
    ]);
    expect(order("command", "descending")).toEqual([
      "loans list",
      "books show",
      "books list",
      "books add",
    ]);
  });
});

describe("figuresSummary", () => {
  it("counts the runs and commands, and the days when the figures name their first", () => {
    const rows = [
      row("books list", figures(2_700)),
      row("books show", figures(27)),
      row("books add", null),
    ];

    expect(figuresSummary(rows, { year: 2026, month: 11, day: 3 }, today)).toBe(
      "2,727 runs of 3 commands, 3 Nov to 10 Nov",
    );
    expect(figuresSummary(rows, null, today)).toBe("2,727 runs of 3 commands");
  });
});

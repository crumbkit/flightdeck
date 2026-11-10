import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { Figures as FiguresOf } from "@/api/model";
import type { FigureRow } from "@/lib/figures";
import { Figures } from "./Figures";

/** Returns figures with `runs` runs, `failed` failures and the rest fixed. */
function figures(runs: number, failed: number, p95_ms: number): FiguresOf {
  return {
    runs,
    help_pages: 0,
    help_before: 3,
    refused: 1,
    failed,
    p50_ms: 12,
    p95_ms,
    p50_out_bytes: 940,
    p95_out_bytes: 2310,
  };
}

const rows: FigureRow[] = [
  { command: "self usage", figures: null, counted: false },
  { command: "books show", figures: figures(611, 48, 64), counted: true },
  { command: "books export", figures: null, counted: true },
  { command: "books list", figures: figures(842, 0, 1240), counted: true },
];

/** Renders the tab, and returns the commands it opens. */
function renderFigures(): string[] {
  const opened: string[] = [];
  render(<Figures name="shelf" rows={rows} onOpen={(command) => opened.push(command)} />);
  return opened;
}

/** Returns the table's rows as the text of each cell. */
function cells(): string[][] {
  const table = screen.getByRole("table", { name: "Figures of each command" });
  return within(table)
    .getAllByRole("row")
    .slice(2)
    .map((row) => [...row.querySelectorAll("th, td")].map((cell) => cell.textContent ?? ""));
}

describe("Figures", () => {
  it("lists every command by runs, most first, with a dash for each figure of a command that never ran and an uncounted command last", () => {
    renderFigures();

    expect(cells()).toEqual([
      ["books list", "842", "0", "1", "3", "12 ms", "1.2 s", "940 B", "2,310 B"],
      ["books show", "611", "48", "1", "3", "12 ms", "64 ms", "940 B", "2,310 B"],
      ["books export", "0", "–", "–", "–", "–", "–", "–", "–"],
      ["self usage", "not counted"],
    ]);
    expect(
      screen.getByText(/^Not counted: flightdeck runs these commands itself to load shelf/),
    ).toBeVisible();
    const table = screen.getByRole("table", { name: "Figures of each command" });
    expect(within(table).getByRole("columnheader", { name: "Runs" })).toHaveAttribute(
      "aria-sort",
      "descending",
    );
    expect(within(table).getByText("48")).toHaveClass("text-destructive");
    for (const zero of within(table).getAllByText("0", { selector: "td" })) {
      expect(zero).not.toHaveClass("text-destructive");
    }
  });

  it("sorts by the header chosen, and reverses on a second press", async () => {
    renderFigures();
    const table = screen.getByRole("table", { name: "Figures of each command" });

    await userEvent.click(within(table).getByRole("button", { name: "Failed" }));

    expect(cells().map((row) => row[0])).toEqual([
      "books show",
      "books list",
      "books export",
      "self usage",
    ]);

    await userEvent.click(within(table).getByRole("button", { name: /^Failed/ }));

    expect(cells().map((row) => row[0])).toEqual([
      "books list",
      "books show",
      "books export",
      "self usage",
    ]);
    expect(within(table).getByRole("columnheader", { name: /^Failed/ })).toHaveAttribute(
      "aria-sort",
      "ascending",
    );
  });

  it("sorts the phone list by the field the Sort by select names", async () => {
    renderFigures();
    const list = screen.getByRole("list", { name: "Figures of each command" });

    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Sort by" }), "command");

    expect(
      within(list)
        .getAllByRole("listitem")
        .map((item) => within(item).getByRole("button").textContent),
    ).toEqual(["books export", "books list", "books show", "self usage"]);
    expect(within(list).getAllByRole("listitem")[0]).toHaveTextContent(
      "books export0 runs–failed–refused–help before– · –wall time, median · p95– · –output, median · p95",
    );
    expect(within(list).getAllByRole("listitem")[3]).toHaveTextContent(/^self usagenot counted$/);
  });

  it("opens the form of the command whose name is pressed", async () => {
    const opened = renderFigures();
    const table = screen.getByRole("table", { name: "Figures of each command" });

    await userEvent.click(within(table).getByRole("button", { name: "books show" }));

    expect(opened).toEqual(["books show"]);
  });
});

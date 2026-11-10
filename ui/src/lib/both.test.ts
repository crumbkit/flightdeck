import { describe, expect, it } from "vitest";
import type { Command, Figures, Tool } from "@/api/model";
import type { Page, WeightReply } from "@/api/weight";
import { command as commandOf, shelf } from "../../tests/fixtures/shelf";
import { bothRows, bothSummary, cornerCounts, heavyBytes, oftenRuns } from "./both";

/** Returns figures with `runs` runs and `p95` bytes of output at the 95th percentile. */
function figures(runs: number, p95: number | null): Figures {
  return {
    runs,
    help_pages: 0,
    help_before: 0,
    refused: 0,
    failed: 0,
    p50_ms: 1,
    p95_ms: 2,
    p50_out_bytes: 1,
    p95_out_bytes: p95,
  };
}

/** Returns shelf's command at `path`, run `runs` times with `p95` bytes of output. */
function ran(path: string, runs: number, p95: number | null = null): Command {
  return {
    ...commandOf(shelf, path),
    runs: commandOf(shelf, path).counted ? runs : 0,
    cost: { figures: figures(runs, p95), findings: null },
  };
}

/**
 * Returns a counted command of the tool's own under `self`, `self update`,
 * run `runs` times, which the server marks built-in by its first word.
 */
function selfUpdate(runs: number): Command {
  return {
    ...ran("books list", runs, 100),
    path: ["self", "update"],
    counted: true,
    built_in: true,
  };
}

/** Returns a page of `path` with `bytes`, or one that timed out when `bytes` is null. */
function page(path: string, bytes: number | null): Page {
  return {
    path: path === "" ? [] : path.split(" "),
    bytes,
    failure: bytes === null ? "timed_out" : null,
    exit: null,
  };
}

/** Pages of 500 B each for the root, books, commands, schemas, self and their commands. */
const pages = [
  "",
  "books",
  "books list",
  "books show",
  "books summarize",
  "books remove",
  "commands",
  "commands list",
  "schemas",
  "schemas list",
  "self",
  "self usage",
].map((path) => page(path, 500));

const reply: WeightReply = { pages, left_out: 0, problem: null };

/** Returns shelf with `commands`. */
function toolOf(commands: Command[]): Tool {
  return { ...shelf, commands };
}

describe("oftenRuns", () => {
  it("is the median runs of the commands that ran, leaving out built-in commands", () => {
    const tool = toolOf([
      ran("books list", 90),
      ran("books show", 10),
      ran("books summarize", 40),
      ran("books remove", 0),
      ran("schemas list", 5000),
    ]);

    expect(oftenRuns(tool)).toBe(40);
  });

  it("leaves out a counted command that the server marks built in", () => {
    const tool = toolOf([ran("books list", 90), ran("books show", 10), selfUpdate(5000)]);

    expect(oftenRuns(tool)).toBe(50);
  });

  it("is the mean of the two middle runs for an even count, and null when nothing ran", () => {
    expect(oftenRuns(toolOf([ran("books list", 90), ran("books show", 10)]))).toBe(50);
    expect(oftenRuns(toolOf([ran("books list", 0)]))).toBeNull();
  });
});

describe("bothRows", () => {
  // Paths: root 500 + group 500 + command 500 + output p95.
  const tool = toolOf([
    ran("books list", 90, 4000), // 5,500 B, often: heavy and often
    ran("books show", 50, 100), // 1,600 B, often: light and often
    ran("books summarize", 10, 3000), // 4,500 B, below the median: no corner
    ran("books remove", 0), // 1,500 B, never: light and never
    ran("self usage", 900, 9000), // built in and not counted, so no output weight
  ]);

  it("multiplies each path's weight by its runs, the most read first, with not counted last", () => {
    const rows = bothRows(tool, reply);

    expect(rows.map((row) => [row.command, row.bytes, row.runs, row.read])).toEqual([
      ["books list", 5500, 90, 495_000],
      ["books show", 1600, 50, 80_000],
      ["books summarize", 4500, 10, 45_000],
      ["books remove", 1500, 0, 0],
      ["self usage", 1500, null, null],
    ]);
  });

  it("puts each counted, measured command that is not built in into its corner", () => {
    const corners = Object.fromEntries(
      bothRows(tool, reply).map((row) => [row.command, row.corner]),
    );

    expect(heavyBytes).toBe(4000);
    expect(corners).toEqual({
      "books list": "heavy_often",
      "books show": "light_often",
      "books summarize": null,
      "books remove": "light_never",
      "self usage": null,
    });
    expect(cornerCounts(bothRows(tool, reply))).toEqual({
      heavy_often: 1,
      light_never: 1,
      heavy_never: 0,
      light_often: 1,
    });
  });

  it("leaves a counted built-in command out of every corner, however heavy and often it runs", () => {
    const rows = bothRows(toolOf([ran("books list", 90, 4000), selfUpdate(5000)]), reply);

    expect(rows.find((row) => row.command === "self update")).toMatchObject({
      runs: 5000,
      corner: null,
    });
  });

  it("lists commands that read the same, such as those that never ran, heaviest first", () => {
    const long: WeightReply = {
      ...reply,
      pages: pages.map((each) =>
        each.path.join(" ") === "books show" ? page("books show", 2500) : each,
      ),
    };

    const rows = bothRows(
      toolOf([ran("books list", 0), ran("books show", 0), ran("books remove", 0)]),
      long,
    );

    // books show's page makes it the heaviest; books list and books remove tie on weight too.
    expect(rows.map((row) => [row.command, row.bytes])).toEqual([
      ["books show", 3500],
      ["books list", 1500],
      ["books remove", 1500],
    ]);
  });

  it("leaves a command whose path has a page that was not measured out of every corner", () => {
    const partial: WeightReply = {
      ...reply,
      pages: pages.map((each) => (each.path.join(" ") === "books" ? page("books", null) : each)),
    };

    const rows = bothRows(toolOf([ran("books list", 90, 9000), ran("books show", 0)]), partial);

    expect(rows.map((row) => [row.command, row.partial, row.corner])).toEqual([
      ["books list", true, null],
      ["books show", true, null],
    ]);
  });

  it("calls a command heavy and never when its help pages reach 4,000 B and it never ran", () => {
    // A command that never ran has no output weight, so its help pages carry it.
    const long: WeightReply = {
      ...reply,
      pages: pages.map((each) =>
        each.path.join(" ") === "books list" ? page("books list", 3000) : each,
      ),
    };

    const rows = bothRows(toolOf([ran("books list", 0), ran("books show", 3)]), long);

    expect(rows.find((row) => row.command === "books list")).toMatchObject({
      bytes: 4000,
      corner: "heavy_never",
    });
  });
});

describe("bothSummary", () => {
  it("names the window and both thresholds", () => {
    const tool = {
      ...toolOf([ran("books list", 90), ran("books show", 10), ran("books summarize", 40)]),
      figures_since: { year: 2026, month: 11, day: 3 },
    };

    expect(bothSummary(tool, { year: 2026, month: 11, day: 10 }).join(" · ")).toBe(
      "Path weight times runs, 3 Nov to 10 Nov · heavy is at least 4,000 B, half the default path budget · often is at least 40 runs, the median of commands that ran",
    );
  });

  it("says no command ran when there is no median", () => {
    expect(
      bothSummary(
        { ...toolOf([ran("books list", 0)]), figures_since: null },
        { year: 2026, month: 11, day: 10 },
      ).join(" · "),
    ).toBe(
      "Path weight times runs · heavy is at least 4,000 B, half the default path budget · no command ran, so none runs often",
    );
  });
});

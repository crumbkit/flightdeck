import { describe, expect, it } from "vitest";
import type { Command, Figures } from "@/api/model";
import type { Page, WeightReply } from "@/api/weight";
import { command as commandOf, shelf } from "../../tests/fixtures/shelf";
import {
  budgets,
  failureWords,
  overWords,
  sortedPaths,
  tokens,
  type WeightPath,
  weightPaths,
  weightSummary,
} from "./weight";

/** Returns a page of `path` with `bytes`. */
function page(path: string, bytes: number): Page {
  return { path: path === "" ? [] : path.split(" "), bytes, failure: null, exit: null };
}

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

/** Returns shelf's command at `path` with `of` as its figures, or none. */
function costed(path: string, of: Figures | null): Command {
  return {
    ...commandOf(shelf, path),
    cost: of === null ? null : { figures: of, findings: null },
  };
}

/** Returns the weight paths of `commands` over `pages`. */
function pathsOf(commands: Command[], pages: Page[]): WeightPath[] {
  const reply: WeightReply = { pages, left_out: 0, problem: null };
  return weightPaths({ ...shelf, commands }, reply);
}

const shelfPages = [page("", 1420), page("books", 960), page("books list", 1180)];

describe("weightPaths", () => {
  it("gives a path the root page, each group page, the command's page and its p95 output, and sums them", () => {
    const [path] = pathsOf([costed("books list", figures(842, 5920))], shelfPages);

    expect(path?.steps.map((step) => [step.label, step.kind, step.bytes, step.budget])).toEqual([
      ["shelf -h", "root", 1420, budgets.root],
      ["shelf books -h", "group", 960, budgets.group],
      ["shelf books list -h", "command", 1180, budgets.command],
      ["output", "output", 5920, null],
    ]);
    expect(path?.total).toBe(9480);
    expect(path?.partial).toBe(false);
    expect(path?.heaviest?.kind).toBe("output");
  });

  it("gives a command that never ran, or that the figures do not count, no output weight and says why", () => {
    const [never, uncounted] = pathsOf(
      [costed("books list", figures(0, 10)), costed("commands list", null)],
      [...shelfPages, page("commands", 430), page("commands list", 980)],
    );

    expect(never?.steps.at(-1)).toMatchObject({ bytes: 0, note: "no runs, so no output weight" });
    expect(never?.total).toBe(3560);
    expect(uncounted?.steps.at(-1)).toMatchObject({ bytes: 0, note: "not counted" });
  });

  it("marks a path partial when one of its pages was not measured, and never calls it over or under", () => {
    const failed: Page = { path: ["books"], bytes: null, failure: "timed_out", exit: null };
    const [path] = pathsOf(
      [costed("books list", figures(5, 9000))],
      [page("", 1420), failed, page("books list", 1180)],
    );

    expect(path?.partial).toBe(true);
    expect(path?.steps[1]).toMatchObject({ bytes: null, note: "not measured: timed out" });
    expect(path?.total).toBe(11_600);
    expect(overWords(path?.total ?? 0, budgets.path, path?.partial)).toBe("partial");
  });

  it("marks a path partial when its page passed the server's cap and has no measurement", () => {
    const [path] = pathsOf([costed("books list", figures(5, 10))], [page("", 1420)]);

    expect(path?.partial).toBe(true);
    expect(path?.steps[2]).toMatchObject({
      bytes: null,
      note: "not measured: past the 400-page cap",
    });
  });
});

describe("overWords", () => {
  it("says over by the bytes past the budget, and under by those left, with the budget itself under", () => {
    expect(overWords(1501, budgets.root)).toBe("over by 1 B");
    expect(overWords(1500, budgets.root)).toBe("0 B under");
    expect(overWords(800, budgets.root)).toBe("700 B under");
  });
});

describe("tokens", () => {
  it("counts a token for each 4 bytes, to the nearest", () => {
    expect([tokens(0), tokens(2), tokens(6), tokens(8000)]).toEqual([0, 1, 2, 2000]);
  });
});

describe("failureWords", () => {
  it("names each reason a page has no bytes", () => {
    const failed = (failure: Page["failure"], exit: number | null = null): Page => ({
      path: [],
      bytes: null,
      failure,
      exit,
    });

    expect(
      [
        failed("exit", 2),
        failed("signal", 9),
        failed("empty"),
        failed("too_large"),
        failed("not_started"),
        failed("not_run"),
      ].map(failureWords),
    ).toEqual([
      "not measured: exited with 2",
      "not measured: ended by signal 9",
      "not measured: printed nothing",
      "over 64 KiB, not a help page",
      "not measured: would not start",
      "not measured: flightdeck stopped measuring after 30 s",
    ]);
  });
});

describe("sortedPaths", () => {
  const paths = pathsOf(
    [costed("books list", figures(5, 100)), costed("books show", figures(5, 9000))],
    [...shelfPages, page("books show", 700)],
  );

  it("sorts by bytes, heaviest first, or by command name", () => {
    expect(sortedPaths(paths, "bytes", "descending").map((path) => path.command)).toEqual([
      "books show",
      "books list",
    ]);
    expect(sortedPaths(paths, "command", "ascending").map((path) => path.command)).toEqual([
      "books list",
      "books show",
    ]);
  });
});

describe("weightSummary", () => {
  it("counts the commands, the pages and the paths over budget, and the commands left out", () => {
    const paths = pathsOf(
      [costed("books list", figures(5, 9000)), costed("books show", figures(5, 10))],
      [...shelfPages, page("books show", 700)],
    );
    const reply = (left_out: number): WeightReply => ({
      pages: [...shelfPages, page("books show", 700)],
      left_out,
      problem: null,
    });

    expect(weightSummary(paths, reply(0))).toBe(
      "2 commands from 4 help pages measured when this view opened · 1 path over budget",
    );
    expect(weightSummary(paths, reply(3))).toBe(
      "2 commands from 4 help pages measured when this view opened · 1 path over budget · 3 commands left out past the 400-page cap",
    );
  });
});

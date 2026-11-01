import { describe, expect, it } from "vitest";
import type { Tool } from "@/api/model";
import { shelf } from "../../tests/fixtures/shelf";
import { search } from "./palette";

function paths(tool: Tool, query: string): string[] {
  return search(tool, query).map((match) => match.command.path.join(" "));
}

function withRuns(runs: Record<string, number>): Tool {
  return {
    ...shelf,
    commands: shelf.commands.map((each) => ({ ...each, runs: runs[each.path.join(" ")] ?? 0 })),
  };
}

describe("search", () => {
  it("finds books show by the caption of its example", () => {
    const [first] = search(shelf, "Show books 2 and 5");

    expect(first?.command.path).toEqual(["books", "show"]);
    expect(first?.example?.command).toBe("shelf books show 2 5");
  });

  it("without runs, ranks an exact path first, then paths, then about text, then examples", () => {
    const quiet = withRuns({});

    expect(paths(quiet, "schemas list")).toEqual(["schemas list"]);
    expect(paths(quiet, "list").slice(0, 4)).toEqual([
      "books list",
      "commands list",
      "schemas list",
      "self figures list",
    ]);
  });

  it("orders two matches of the same rank by runs", () => {
    const busy = withRuns({ "commands list": 2, "self figures list": 9 });

    expect(paths(busy, "list").slice(0, 4)).toEqual([
      "self figures list",
      "commands list",
      "books list",
      "schemas list",
    ]);
  });

  it("ranks path matches above an about-text match with more runs", () => {
    // "books show" and "schemas show" match "show" by path; "books remove"
    // matches only in its about text, "...so list and show skip them".
    const busy = withRuns({ "books remove": 50 });

    expect(paths(busy, "show")).toEqual(["books show", "schemas show", "books remove"]);
  });

  it("ranks books list above commands list for list in a freshly opened shelf", () => {
    expect(paths(shelf, "list").slice(0, 2)).toEqual(["books list", "commands list"]);
  });

  it("lists every command in the guide's order for an empty query", () => {
    expect(paths(withRuns({ "self usage": 99 }), "  ")).toEqual(
      shelf.commands.map((each) => each.path.join(" ")),
    );
  });
});

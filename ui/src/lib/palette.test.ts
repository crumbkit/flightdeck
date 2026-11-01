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

  it("ranks an exact path match above an about-text match with more runs", () => {
    // "books list" is an exact path; "books show" matches only in its about
    // text, "Show the books with the given ids".
    const busy = withRuns({ "books show": 50 });

    expect(paths(busy, "books list")[0]).toBe("books list");
    expect(paths(busy, "the books")).toEqual([
      "books show",
      "books list",
      "books summarize",
      "books remove",
      "commands list",
      "schemas show",
    ]);
  });

  it("lists every command in the guide's order for an empty query", () => {
    expect(paths(withRuns({ "self usage": 99 }), "  ")).toEqual(
      shelf.commands.map((each) => each.path.join(" ")),
    );
  });
});

import { describe, expect, it } from "vitest";
import { command, shelf, shelfWithGoals } from "../../tests/fixtures/shelf";
import { effectOf, railGroups, whyNotRunnable } from "./rail";

function titles(groups: ReturnType<typeof railGroups>) {
  return groups.map((group) => [group.title, group.commands.map((each) => each.path.join(" "))]);
}

describe("railGroups", () => {
  it("groups the commands under the goal headings in help order", () => {
    expect(titles(railGroups(shelfWithGoals))).toEqual([
      ["Browse the catalogue", ["books list", "books show", "books summarize", "books remove"]],
      ["Explore the commands", ["commands list", "schemas list", "schemas show"]],
      ["Review how shelf is used", ["self usage", "self figures list"]],
    ]);
  });

  it("lists every command under no heading when the tool prints no goals", () => {
    const groups = railGroups(shelf);

    expect(groups).toHaveLength(1);
    expect(groups[0]?.title).toBeNull();
    expect(groups[0]?.commands).toHaveLength(9);
  });

  it("keeps the guide's order whatever the runs counts are", () => {
    const busy = {
      ...shelfWithGoals,
      commands: shelfWithGoals.commands.map((each) => ({ ...each, runs: 100 - each.runs })),
    };
    busy.commands.reverse();

    expect(titles(railGroups(busy))).toEqual(titles(railGroups(shelfWithGoals)));
  });
});

describe("effectOf", () => {
  it("names each command's effect with its word", () => {
    expect(effectOf(command(shelf, "books list"))).toBe("read-only");
    expect(effectOf(command(shelf, "books remove"))).toBe("destructive");
    expect(effectOf({ ...command(shelf, "books list"), effects: null })).toBe("effects unknown");
    expect(
      effectOf({
        ...command(shelf, "books list"),
        effects: { read_only: false, destructive: false, idempotent: false },
      }),
    ).toBe("additive");
  });
});

describe("whyNotRunnable", () => {
  it("runs a read-only command and holds back one that needs a confirmation", () => {
    expect(whyNotRunnable(command(shelf, "books list"), false)).toBeNull();
    expect(whyNotRunnable(command(shelf, "books remove"), false)).toContain("preview and confirm");
  });

  it("holds back every command but the read-only ones under --read-only", () => {
    const additive = {
      ...command(shelf, "books list"),
      effects: { read_only: false, destructive: false, idempotent: true },
    };

    expect(whyNotRunnable(additive, true)).toContain("--read-only");
    expect(whyNotRunnable(command(shelf, "books list"), true)).toBeNull();
  });
});

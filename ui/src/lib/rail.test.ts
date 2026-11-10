import { describe, expect, it } from "vitest";
import { command, shelf, shelfWithoutGoals } from "../../tests/fixtures/shelf";
import { effectOf, needsConfirmation, railGroups, whyNotRunnable } from "./rail";

function titles(groups: ReturnType<typeof railGroups>) {
  return groups.map((group) => [group.title, group.commands.map((each) => each.path.join(" "))]);
}

describe("railGroups", () => {
  it("groups the commands under the goal headings in help order", () => {
    expect(titles(railGroups(shelf))).toEqual([
      ["Browse the catalogue", ["books list", "books show", "books summarize", "books remove"]],
      ["Explore the commands", ["commands list", "schemas list", "schemas show"]],
      ["Review how shelf is used", ["self usage", "self figures list", "self runs list"]],
    ]);
  });

  it("lists every command under no heading when the tool prints no goals", () => {
    const groups = railGroups(shelfWithoutGoals);

    expect(groups).toHaveLength(1);
    expect(groups[0]?.title).toBeNull();
    expect(groups[0]?.commands).toHaveLength(10);
  });

  it("keeps the guide's order whatever the runs counts are", () => {
    const busy = {
      ...shelf,
      commands: shelf.commands.map((each) => ({ ...each, runs: 100 - each.runs })),
    };
    busy.commands.reverse();

    expect(titles(railGroups(busy))).toEqual(titles(railGroups(shelf)));
  });
});

describe("effectOf", () => {
  it("names each command's effect with its word", () => {
    expect(effectOf(command(shelf, "books list"))).toBe("read-only");
    expect(effectOf(command(shelf, "books remove"))).toBe("destructive");
    expect(effectOf({ ...command(shelf, "books list"), class: "destructive", effects: null })).toBe(
      "effects unknown",
    );
    expect(
      effectOf({
        ...command(shelf, "books list"),
        class: "additive",
        effects: { read_only: false, destructive: false, idempotent: false },
      }),
    ).toBe("additive");
  });
});

describe("a command that declares itself read-only and destructive", () => {
  const both = {
    ...command(shelf, "books list"),
    class: "destructive" as const,
    effects: { read_only: true, destructive: true, idempotent: false },
  };

  it("takes the class that the server gave it", () => {
    expect(effectOf(both)).toBe("destructive");
    expect(needsConfirmation(both)).toBe(true);
  });

  it("is held back under --read-only", () => {
    expect(whyNotRunnable(both, true)).toBe(
      "Turned off by --read-only. Start flightdeck without it to run this command.",
    );
  });
});

describe("whyNotRunnable", () => {
  it("runs every command when flightdeck was started without --read-only", () => {
    expect(whyNotRunnable(command(shelf, "books list"), false)).toBeNull();
    expect(whyNotRunnable(command(shelf, "books remove"), false)).toBeNull();
  });

  it("holds back every command but the read-only ones under --read-only", () => {
    const additive = {
      ...command(shelf, "books list"),
      class: "additive" as const,
      effects: { read_only: false, destructive: false, idempotent: true },
    };

    expect(whyNotRunnable(additive, true)).toBe(
      "Turned off by --read-only. Start flightdeck without it to run this command.",
    );
    expect(whyNotRunnable(command(shelf, "books list"), true)).toBeNull();
  });
});

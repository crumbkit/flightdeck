import { describe, expect, it } from "vitest";
import type { RunEvent } from "@/api/runs";
import { command, shelf } from "../../tests/fixtures/shelf";
import {
  asksForYes,
  confirmedLine,
  confirmLabel,
  countOf,
  effectsInWords,
  question,
} from "./confirm";

const remove = command(shelf, "books remove");

describe("confirmLabel", () => {
  it("names the verb, the count and the resource in the singular for one row", () => {
    expect(confirmLabel(remove, 1)).toBe("Remove 1 book");
  });

  it("names the resource in the plural for more rows", () => {
    expect(confirmLabel(remove, 3)).toBe("Remove 3 books");
  });

  it("names only the verb when the preview counted nothing", () => {
    expect(confirmLabel(remove, null)).toBe("Remove");
  });

  it("counts items for a command whose path is only its verb", () => {
    expect(confirmLabel({ ...remove, path: ["sweep"] }, 2)).toBe("Sweep 2 items");
  });
});

describe("question", () => {
  it("asks with the verb and the count", () => {
    expect(question(remove, 1, "shelf books remove 2 --yes")).toBe("Remove 1 book?");
  });

  it("asks to run the exact line when the preview counted nothing", () => {
    expect(question(remove, null, "shelf books remove 2 --yes")).toBe(
      "Run shelf books remove 2 --yes?",
    );
  });
});

describe("effectsInWords", () => {
  it("words each effect the command declares", () => {
    expect(effectsInWords(remove)).toBe("Destructive. Idempotent.");
    expect(
      effectsInWords({
        ...remove,
        effects: { read_only: false, destructive: false, idempotent: false },
      }),
    ).toBe("Additive. Not idempotent.");
  });

  it("says that a command without effects counts as destructive", () => {
    expect(effectsInWords({ ...remove, effects: null })).toBe(
      "Effects unknown, so flightdeck treats this command as destructive.",
    );
  });
});

describe("confirmedLine", () => {
  it("adds --yes for a command that takes it", () => {
    expect(confirmedLine(remove, "shelf books remove 2")).toBe("shelf books remove 2 --yes");
  });

  it("leaves the line as it is for a command without --yes", () => {
    const withoutYes = {
      ...remove,
      arguments: remove.arguments.filter((each) => each.crumbkit_flag !== "--yes"),
    };

    expect(confirmedLine(withoutYes, "shelf books remove 2")).toBe("shelf books remove 2");
  });
});

describe("asksForYes", () => {
  const ended = (code: string): RunEvent => ({
    event: "ended",
    outcome: { kind: "tool_failure", failure: { code } },
    exit: 1,
    duration_ms: 3,
  });

  it("finds crumbkit's needs_yes failure at the end of a run", () => {
    expect(asksForYes(ended("shelf::confirmation::needs_yes"))).toBe(true);
  });

  it("ignores every other failure", () => {
    expect(asksForYes(ended("shelf::books::not_found"))).toBe(false);
  });
});

describe("countOf", () => {
  it("counts the rows of a listing and one for a record", () => {
    const row = { id: 2, title: "Frankenstein" };
    const listing = JSON.stringify({ schema: "s", data: [row, row], total: 2, truncated: false });

    expect(countOf([listing])).toBe(2);
    expect(countOf([JSON.stringify({ schema: "s", dry_run: true, data: [row] })])).toBe(1);
  });

  it("counts nothing for prose", () => {
    expect(countOf([JSON.stringify({ schema: "s", data: "Done." })])).toBeNull();
  });
});

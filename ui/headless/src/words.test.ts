import { describe, expect, it } from "vitest";
import { CELL_STATES_WORST_FIRST, type ReportState } from "./document.ts";
import {
  ageWord,
  cadenceWord,
  cellWord,
  checkerWord,
  dayWord,
  moment,
  mono,
  numberWord,
  phraseText,
  reportWord,
  rowName,
  shortCommit,
  shortMoment,
  shortName,
  spokenState,
} from "./words.ts";

describe("ageWord", () => {
  it("gives minutes under an hour, hours under a day and days after", () => {
    expect(ageWord(20)).toBe("under 1 min");
    expect(ageWord(240)).toBe("4 min");
    expect(ageWord(3600)).toBe("1 h");
    expect(ageWord(3 * 3600 + 59)).toBe("3 h");
    expect(ageWord(86_400)).toBe("1 d");
    expect(ageWord(9 * 86_400 + 3600)).toBe("9 d");
  });
});

describe("numberWord", () => {
  it("groups thousands with commas", () => {
    expect(numberWord(1184)).toBe("1,184");
    expect(numberWord(49_152)).toBe("49,152");
    expect(numberWord(7)).toBe("7");
  });
});

describe("moment", () => {
  it("names the hour in UTC and the day", () => {
    expect(moment("2026-11-25T06:00:00Z")).toBe("06:00 UTC on 25 November 2026");
    expect(moment("2026-11-25T23:13:20Z")).toBe("23:13 UTC on 25 November 2026");
  });
});

describe("every state has a word (BASE-6)", () => {
  it("gives each cell state a word and a spoken word", () => {
    for (const state of CELL_STATES_WORST_FIRST) {
      expect(cellWord(state, null)).not.toBe("");
      expect(spokenState(state, null)).not.toBe("");
    }
  });

  it("gives each report state a word", () => {
    const states: ReportState[] = ["current", "stale", "no_report", "invalid_report"];
    expect(states.map(reportWord)).toEqual(["current", "stale", "no report", "invalid report"]);
  });

  it("calls a cell of a column with no report by that, and an absent cell a dash it speaks", () => {
    expect(cellWord("not_readable", "flightdeck::compliance::no_report")).toBe("no report");
    expect(cellWord("not_readable", "flightdeck::compliance::invalid_report")).toBe(
      "invalid report",
    );
    expect(cellWord("not_readable", "board::ref::dangling")).toBe("not readable");
    expect(cellWord("absent", null)).toBe("–");
    expect(spokenState("absent", null)).toBe("does not apply");
  });
});

describe("checkerWord", () => {
  it("names a budget's unit and, in the long form, its max", () => {
    const perFile = { check: "budget", max: 49_152, per: "file", unit: "bytes" };
    expect(checkerWord(perFile, false)).toBe("budget · bytes per file");
    expect(checkerWord(perFile, true)).toBe("budget · bytes per file · max 49,152");
    expect(checkerWord({ check: "budget", max: 1000, unit: "words" }, true)).toBe(
      "budget · words · max 1,000",
    );
  });

  it("names a version's minimum, a zero rule's window, a rubric and a rule with no checker", () => {
    expect(checkerWord({ check: "version", at_least: "1.99" }, false)).toBe("version");
    expect(checkerWord({ check: "version", at_least: "1.99" }, true)).toBe(
      "version · at least 1.99",
    );
    expect(checkerWord({ check: "zero", within_days: 7 }, false)).toBe("zero · within 7 d");
    expect(checkerWord({ check: "judged", rubric: "/rubrics/plain.md" }, true)).toBe(
      "judged · /rubrics/plain.md",
    );
    expect(checkerWord(null, true)).toBe("written");
  });

  it("adds what a pattern quotes in the long form, as the Rules page shows it", () => {
    expect(checkerWord({ check: "pattern", quote: "match-only" }, true)).toBe(
      "pattern · match-only",
    );
  });

  it("names a pattern as itself and a checker it does not know as an unknown checker", () => {
    expect(checkerWord({ check: "pattern" }, true)).toBe("pattern");
    expect(checkerWord({ check: "pattern", quote: "match-only" }, false)).toBe("pattern");
    expect(checkerWord({ check: "regex" }, false)).toBe("unknown checker regex");
  });
});

describe("names", () => {
  it("names a rule by its requirement, and a row with none by its path", () => {
    const row = { id: "/rules/rust/file-size", requirement: "Keep each Rust file small." };
    expect(rowName(row)).toBe("Keep each Rust file small");
    expect(rowName({ id: "/rules/x", requirement: null })).toBe("/rules/x");
  });

  it("shortens a commit and a path", () => {
    expect(shortCommit("d4646834fbc9acd2642854b48b467cb5cf146570")).toBe("d464683");
    expect(shortName("/rules/rust/file-size")).toBe("file-size");
    expect(shortName("owner/app")).toBe("app");
  });
});

describe("shortMoment", () => {
  it("names a time by its day, short month, year and UTC time", () => {
    expect(shortMoment("2026-11-25T06:00:00Z")).toBe("25 Nov 2026, 06:00 UTC");
    expect(shortMoment("2026-09-03T23:05:00+01:00")).toBe("3 Sep 2026, 22:05 UTC");
  });
});

describe("phrases", () => {
  it("reads a phrase as one string, with its mono pieces as plain text", () => {
    expect(phraseText([mono(".nvmrc"), " line 1 pins 20.18.0."])).toBe(
      ".nvmrc line 1 pins 20.18.0.",
    );
    expect(phraseText([])).toBe("");
  });
});

describe("cadenceWord", () => {
  it("words a job's interval as hourly, daily, weekly, or every so many hours or days", () => {
    expect([1, 6, 24, 48, 168, 30].map(cadenceWord)).toEqual([
      "every hour",
      "every 6 hours",
      "daily",
      "every 2 days",
      "weekly",
      "every 30 hours",
    ]);
  });
});

describe("dayWord", () => {
  it("names a time's day and month in UTC, such as 16 November", () => {
    expect(dayWord("2026-11-16T23:59:00Z")).toBe("16 November");
    expect(dayWord("2026-11-16T23:59:00-02:00")).toBe("17 November");
  });
});

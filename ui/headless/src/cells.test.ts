import { describe, expect, it } from "vitest";
import {
  failureDetail,
  failurePhrase,
  measuredPhrase,
  measuredText,
  stalePhrase,
} from "./cells.ts";
import type { Cell, Row } from "./document.ts";

const facts = {
  used: 1184,
  max: 1000,
  files: ["/AGENTS.md", "/CONTEXT.md"],
  over: [],
  behind: [],
  omitted: { files: 0, over: 0, behind: 0 },
  zero: null,
};

const row: Row = {
  id: "/rules/agents/context-budget",
  section: "measured",
  source: "imposed",
  requirement: "Agent context stays under 1,000 words.",
  purpose: null,
  kind: "standing",
  show: "failing",
  checker: { check: "budget", max: 1000, unit: "words" },
  groups: [],
  cases: null,
  cells: [],
};

const cell: Cell = {
  repository: "owner/app",
  state: "failing",
  code: null,
  result: "over",
  facts,
  run: null,
  age_s: null,
};

describe("the cells' sentences, as the artboards word them", () => {
  it("names the files a whole-repository budget counted", () => {
    expect(failureDetail(row, cell)).toBe("AGENTS.md and CONTEXT.md hold 1,184 words together.");
    const one = { ...cell, facts: { ...facts, files: ["/AGENTS.md"] } };
    expect(failureDetail(row, one)).toBe("AGENTS.md holds 1,184 words, and the limit is 1,000.");
  });

  it("names two files and counts the rest when the build left some out of the list", () => {
    const files = Array.from({ length: 20 }, (_, at) => `/f${at}.md`);
    const capped = {
      ...cell,
      facts: { ...facts, files, omitted: { ...facts.omitted, files: 30 } },
    };
    expect(failureDetail(row, capped)).toBe(
      "f0.md, f1.md and 48 more files hold 1,184 words together.",
    );
  });

  it("says a pin within a version rule is at least its minimum, because the report gives no version found", () => {
    const version: Row = { ...row, checker: { check: "version", at_least: "1.99" } };
    const within = {
      ...cell,
      state: "current" as const,
      result: "within",
      facts: { ...facts, used: null, max: null, files: ["/rust-toolchain.toml"] },
    };
    expect(measuredText(version, within)).toBe("rust-toolchain.toml pins at least 1.99");
    const two = {
      ...within,
      facts: { ...within.facts, files: ["/rust-toolchain.toml", "/mise.toml"] },
    };
    expect(measuredText(version, two)).toBe("rust-toolchain.toml and mise.toml pin at least 1.99");
  });

  it("says what a pin behind its rule pins, and leaves the minimum to the rule's name", () => {
    const version: Row = { ...row, checker: { check: "version", at_least: "22.12" } };
    const behind = {
      ...cell,
      result: "behind",
      facts: {
        ...facts,
        used: null,
        max: null,
        files: ["/.nvmrc"],
        behind: [{ file: "/.nvmrc", line: 1, found: "20.18.0" }],
      },
    };
    expect(failureDetail(version, behind)).toBe(".nvmrc line 1 pins 20.18.0.");
  });
});

describe("the cells' phrases, which set paths, commits and measures in the mono face", () => {
  it("marks the files and the measures of a failure", () => {
    expect(failurePhrase(row, cell)).toEqual([
      { mono: "AGENTS.md" },
      " and ",
      { mono: "CONTEXT.md" },
      " hold 1,184 words together.",
    ]);
    const zeroRow: Row = { ...row, section: "zero", checker: { check: "zero", within_days: 7 } };
    const notZero = {
      ...cell,
      result: "not_zero",
      facts: {
        ...facts,
        files: [],
        zero: {
          suite: "/suites/callers",
          run: null,
          age_s: 3600,
          measures: { remaining: 3, unknown: 0 },
        },
      },
    };
    expect(failurePhrase(zeroRow, notZero)).toEqual([
      "The newest run reports ",
      { mono: "remaining 3" },
      " and ",
      { mono: "unknown 0" },
      ".",
    ]);
  });

  it("marks the suite and the commit of a stale suite", () => {
    const suite: Row = { ...row, id: "/suites/app-rust-standard", section: "suite", checker: null };
    const never = { ...cell, state: "stale" as const, result: null, facts: null, run: null };
    expect(stalePhrase(suite, never)).toEqual([
      "The suite ",
      { mono: "/suites/app-rust-standard" },
      " has no clean full run yet.",
    ]);
    const ran = {
      ...never,
      age_s: 12 * 86_400,
      run: {
        id: "/suites/app-rust-standard/runs/1",
        finished: null,
        commit: "b07e1d5aa",
        outcome: "passed",
        failed: [],
        commits_since: 19,
        commits_since_code: null,
      },
    };
    expect(stalePhrase(suite, ran)).toEqual([
      "Last clean run 12 days ago at ",
      { mono: "b07e1d5" },
      ", with 19 commits since.",
    ]);
  });

  it("marks the file a budget found over its limit", () => {
    const perFile: Row = {
      ...row,
      checker: { check: "budget", max: 40_960, unit: "bytes", per: "file" },
    };
    const over = {
      ...cell,
      facts: {
        ...facts,
        used: 46_880,
        over: [{ file: "/pipeline/transform/joins.py", used: 46_880 }],
      },
    };
    expect(measuredPhrase(perFile, over)).toEqual([
      "1 file over: ",
      { mono: "pipeline/transform/joins.py" },
      " 46,880",
    ]);
  });
});

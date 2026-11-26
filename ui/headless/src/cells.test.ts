import { describe, expect, it } from "vitest";
import { failureDetail, measuredText } from "./cells.ts";
import type { Cell, Row } from "./document.ts";

const facts = {
  used: 1184,
  max: 1000,
  files: ["/AGENTS.md", "/CLAUDE.md"],
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
    expect(failureDetail(row, cell)).toBe("/AGENTS.md and /CLAUDE.md hold 1,184 words together.");
    const one = { ...cell, facts: { ...facts, files: ["/AGENTS.md"] } };
    expect(failureDetail(row, one)).toBe("/AGENTS.md holds 1,184 words, and the limit is 1,000.");
  });

  it("says a pin within a version rule is at least its minimum, because the report gives no version found", () => {
    const version: Row = { ...row, checker: { check: "version", at_least: "1.99" } };
    const within = {
      ...cell,
      state: "current" as const,
      result: "within",
      facts: { ...facts, used: null, max: null, files: ["/rust-toolchain.toml"] },
    };
    expect(measuredText(version, within)).toBe("/rust-toolchain.toml pins at least 1.99");
  });
});

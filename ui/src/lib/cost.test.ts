import { describe, expect, it } from "vitest";
import type { Cost, Finding } from "@/api/model";
import {
  badgeOf,
  bytesWords,
  dayWords,
  percentOf,
  retriedCodes,
  runsWords,
  savedCalls,
  tokensWords,
} from "./cost";

/** Returns a finding of `kind` that counts `runs`, with `code` when given. */
function finding(kind: string, runs: number, code: string | null = null): Finding {
  return { finding: kind, runs, suggestion: "", code };
}

const figures: Cost["figures"] = {
  runs: 86,
  help_pages: 19,
  help_before: 11,
  refused: 0,
  failed: 31,
  p95_ms: 65_100,
  p95_out_bytes: 1384,
};

describe("runsWords", () => {
  it("names the runs and the day the figures start", () => {
    expect(runsWords(86, "2026-10-27")).toBe("86 runs since 27 Oct");
  });

  it("names one run in the singular and leaves out a day the tool does not name", () => {
    expect(runsWords(1, null)).toBe("1 run");
  });
});

describe("dayWords", () => {
  it("writes a day as its date and the month's short name", () => {
    expect(dayWords("2026-11-03")).toBe("3 Nov");
  });
});

describe("output in bytes and tokens", () => {
  it("groups the bytes and counts a token for each four bytes, rounded", () => {
    expect(bytesWords(1384)).toBe("1,384 B");
    expect(tokensWords(1384)).toBe("about 346 tokens");
    expect(tokensWords(142)).toBe("about 36 tokens");
  });
});

describe("percentOf", () => {
  it("rounds to a whole percentage and has none of no runs", () => {
    expect(percentOf(31, 86)).toBe(36);
    expect(percentOf(0, 0)).toBeNull();
  });
});

describe("savedCalls", () => {
  it("counts retries and repeats from the findings and the help and usage errors from the figures", () => {
    const findings = [
      finding("retry after failure", 11, "process::exited"),
      finding("retry after failure", 7, "workspace::not_current"),
      finding("identical repeat", 18),
      finding("slow command", 4),
    ];

    expect(savedCalls({ figures, findings })).toEqual([
      { label: "Ran again after a failure", count: 18 },
      { label: "Same call again within 60 s", count: 18 },
      { label: "Read help before running", count: 11 },
      { label: "Help pages read", count: 19 },
      { label: "Usage errors", count: 0 },
    ]);
  });

  it("leaves out the counts that the tool does not report", () => {
    const older = { ...figures, help_before: null, refused: null };

    expect(savedCalls({ figures: older, findings: null })).toEqual([
      { label: "Help pages read", count: 19 },
    ]);
  });
});

describe("retriedCodes", () => {
  it("sums the retries of each failure's code, most retried first", () => {
    const findings = [
      finding("retry after failure", 3, "process::not_ready"),
      finding("retry after failure", 11, "process::exited"),
      finding("retry after failure", 2, "process::not_ready"),
      finding("identical repeat", 9),
    ];

    expect(retriedCodes(findings)).toEqual([
      { code: "process::exited", runs: 11 },
      { code: "process::not_ready", runs: 5 },
    ]);
  });

  it("has no codes without findings", () => {
    expect(retriedCodes(null)).toEqual([]);
  });
});

describe("badgeOf", () => {
  it("shortens a slow command and a retry, and keeps any other finding's own words", () => {
    expect(badgeOf(finding("slow command", 1))).toEqual({ label: "slow", tone: "slow" });
    expect(badgeOf(finding("retry after failure", 1))).toEqual({ label: "retry", tone: "waste" });
    expect(badgeOf(finding("help loop", 1))).toEqual({ label: "help loop", tone: "waste" });
  });
});

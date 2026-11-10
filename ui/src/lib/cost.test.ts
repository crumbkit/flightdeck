import { describe, expect, it } from "vitest";
import type { Cost, Finding, Run } from "@/api/model";
import {
  bytesWords,
  commandHistory,
  dayOf,
  dayWords,
  percentOf,
  periodWords,
  savedCalls,
  stripFindings,
  tokensWords,
  topCodes,
  usageCommand,
} from "./cost";

/** Returns a finding of `kind` that counts `runs`, with `code` when given. */
function finding(kind: Finding["kind"], runs: number, code: string | null = null): Finding {
  return { kind, runs, code, p95_ms: null, p95_out_bytes: null };
}

const figures: Cost["figures"] = {
  runs: 86,
  help_pages: 19,
  help_before: 11,
  refused: 0,
  failed: 31,
  p50_ms: 5900,
  p95_ms: 65_100,
  p50_out_bytes: 142,
  p95_out_bytes: 1384,
};

const today = { year: 2026, month: 11, day: 10 };

describe("periodWords", () => {
  it("names the runs and the days from the figures' first to today", () => {
    expect(periodWords(86, { year: 2026, month: 10, day: 27 }, today)).toBe(
      "86 runs, 27 Oct to 10 Nov",
    );
  });

  it("names one run in the singular and leaves out the days when the tool names none", () => {
    expect(periodWords(1, null, today)).toBe("1 run");
  });

  it("names the sessions when the tool lists its runs", () => {
    expect(periodWords(86, { year: 2026, month: 10, day: 27 }, today, 5)).toBe(
      "86 runs in 5 sessions, 27 Oct to 10 Nov",
    );
    expect(periodWords(1, null, today, 1)).toBe("1 run in 1 session");
  });
});

describe("commandHistory", () => {
  /** Returns a run of `command` in `session` that exited with `exit` and `code`. */
  function run(command: string, session: number, exit: number | null, code: string | null): Run {
    return {
      session,
      step: 1,
      at: 0,
      caller: "claude-code",
      command,
      flags: [],
      help: false,
      exit,
      code,
      ms: 1,
      out_bytes: null,
    };
  }

  it("counts the command's sessions and its failures by code, most first, without usage errors or help", () => {
    const history = [
      run("up", 1, 1, "process::exited"),
      run("up", 1, 1, "process::not_ready"),
      run("up", 2, 1, "process::exited"),
      run("up", 2, 2, "cli::usage"),
      run("up", 3, 0, null),
      { ...run("up", 4, 0, null), help: true },
      run("down", 5, 1, "process::exited"),
    ];

    expect(commandHistory(history, "up")).toEqual({
      sessions: 4,
      codes: [
        { code: "process::exited", count: 2 },
        { code: "process::not_ready", count: 1 },
      ],
    });
  });

  it("opens the usage view on the command in the form when it ran, else on the one that ran most", () => {
    const history = [
      run("down", 1, 0, null),
      run("up", 1, 0, null),
      run("up", 2, 0, null),
      { ...run("new", 2, 0, null), help: true },
    ];

    expect(usageCommand(history, "down")).toBe("down");
    expect(usageCommand(history, "new")).toBe("up");
    expect(usageCommand(history, null)).toBe("up");
    expect(usageCommand([], "new")).toBe("new");
  });

  it("folds the codes after the first three into one row", () => {
    const codes = ["a", "b", "c", "d", "e"].map((code, index) => ({ code, count: 5 - index }));

    expect(topCodes(codes)).toEqual([
      { code: "a", count: 5 },
      { code: "b", count: 4 },
      { code: "c", count: 3 },
      { code: null, count: 3, others: 2 },
    ]);
  });
});

describe("days", () => {
  it("writes a day as its date and the month's short name, and reads one from a local date", () => {
    expect(dayWords({ year: 2026, month: 11, day: 3 })).toBe("3 Nov");
    expect(dayOf(new Date(2026, 0, 31, 23, 59))).toEqual({ year: 2026, month: 1, day: 31 });
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
      finding("retry_after_failure", 11, "process::exited"),
      finding("retry_after_failure", 7, "workspace::not_current"),
      finding("identical_repeat", 18),
      finding("slow_command", 4),
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

describe("stripFindings", () => {
  it("keeps a slow command, a help loop, the most retried failure and heavy output, in that order", () => {
    const findings = [
      finding("heavy_output", 3),
      finding("identical_repeat", 18),
      finding("retry_after_failure", 7, "workspace::not_current"),
      finding("retry_after_failure", 11, "process::exited"),
      finding("usage_error", 2),
      finding("other", 5),
      finding("help_loop", 9),
      finding("slow_command", 4),
    ];

    expect(stripFindings(findings).map((each) => [each.kind, each.code])).toEqual([
      ["slow_command", null],
      ["help_loop", null],
      ["retry_after_failure", "process::exited"],
      ["heavy_output", null],
    ]);
  });

  it("has nothing to phrase without findings", () => {
    expect(stripFindings(null)).toEqual([]);
  });
});

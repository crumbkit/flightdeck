import { describe, expect, it } from "vitest";
import type { Argument, Run } from "@/api/model";
import {
  flagRows,
  gapWords,
  kindsOf,
  loops,
  neighbours,
  sessionsOf,
  spanWords,
  stepsMatrix,
} from "./usage";

/** Noon on 7 Nov 2026 in the browser's time zone, in seconds. */
const noon = new Date(2026, 10, 7, 12, 0, 0).getTime() / 1000;

/** Returns a run that succeeded, changed by `fields`. */
function run(fields: Partial<Run>): Run {
  return {
    session: 1,
    step: 1,
    at: noon,
    caller: "claude-code",
    command: "up",
    flags: [],
    help: false,
    exit: 0,
    code: null,
    ms: 5,
    out_bytes: 10,
    ...fields,
  };
}

/** Returns the runs of one session, one second apart, from commands and their changes. */
function session(number: number, each: Partial<Run>[]): Run[] {
  return each.map((fields, index) =>
    run({ session: number, step: index + 1, at: noon + index, ...fields }),
  );
}

/** Returns a named flag argument of a command's model. */
function flag(name: string, fields: Partial<Argument> = {}): Argument {
  return {
    name,
    short: null,
    kind: "named",
    arity: { kind: "flag" },
    required: false,
    defaults: [],
    possible_values: [],
    help: { line: null, long: null },
    one_of: [],
    crumbkit_flag: null,
    global: false,
    ...fields,
  };
}

describe("kindsOf", () => {
  it("marks help pages, usage errors, failures, retries after a failure and the same call again", () => {
    const runs = session(1, [
      { help: true },
      { exit: 2, code: "cli::usage" },
      { exit: 1, code: "process::exited" },
      {},
      { flags: ["--json"] },
      { flags: ["--json"] },
      { command: "down" },
      { exit: null },
    ]);

    expect(kindsOf(runs)).toEqual(["help", "usage", "fail", "retry", "ok", "repeat", "ok", "fail"]);
  });

  it("counts neither a retry nor a repeat across sessions or after 60 seconds", () => {
    const runs = [
      run({ session: 1, step: 1, exit: 1 }),
      run({ session: 2, step: 1 }),
      run({ session: 2, step: 2, at: noon + 61 }),
    ];

    expect(kindsOf(runs)).toEqual(["fail", "ok", "ok"]);
  });
});

describe("sessionsOf", () => {
  it("groups the runs by session with their kinds, the most recent session first", () => {
    const runs = [
      ...session(1, [{ exit: 1 }, {}]),
      ...session(2, [{ caller: "person", at: noon + 3600 }]),
    ];

    const sessions = sessionsOf(runs);

    expect(sessions.map((each) => [each.number, each.caller, each.kinds])).toEqual([
      [2, "person", ["ok"]],
      [1, "claude-code", ["fail", "retry"]],
    ]);
    expect(sessions[1]?.first).toBe(noon);
    expect(sessions[1]?.last).toBe(noon + 1);
  });
});

describe("spanWords and gapWords", () => {
  it("names the second day only when a session crosses midnight", () => {
    const evening = new Date(2026, 10, 7, 21, 53).getTime() / 1000;
    const late = new Date(2026, 10, 7, 23, 9).getTime() / 1000;
    const morning = new Date(2026, 10, 8, 0, 28).getTime() / 1000;

    expect(spanWords(evening, late)).toBe("7 Nov 21:53 to 23:09");
    expect(spanWords(evening, morning)).toBe("7 Nov 21:53 to 8 Nov 00:28");
  });

  it("marks a new day, a pause of 10 minutes or more, and nothing for a shorter one", () => {
    const morning = new Date(2026, 10, 8, 10, 58).getTime() / 1000;

    expect(gapWords(noon, noon + 9 * 60)).toBeNull();
    expect(gapWords(noon, noon + 20 * 60)).toBe("20 min");
    expect(gapWords(noon, noon + 150 * 60)).toBe("2.5 h");
    expect(gapWords(noon, morning)).toBe("8 Nov");
  });
});

describe("stepsMatrix", () => {
  it("counts which command followed which in a session, with help pages as one command", () => {
    const runs = [
      ...session(1, [{ command: "new" }, {}, { command: "status" }, {}]),
      ...session(2, [{ help: true, command: "new" }, { command: "new" }]),
    ];

    const matrix = stepsMatrix(runs, 11);

    // Commands with more runs come first, ties by name, and help pages last.
    expect(matrix.keys).toEqual(["new", "up", "status", "help"]);
    expect(matrix.cells).toEqual([
      [0, 1, 0, 0],
      [0, 0, 1, 0],
      [0, 1, 0, 0],
      [1, 0, 0, 0],
    ]);
    expect([matrix.shown, matrix.total]).toEqual([4, 4]);
  });

  it("keeps the commands with the most runs and counts the steps it leaves out", () => {
    const runs = session(1, [{}, {}, { command: "down" }, { command: "logs" }]);

    const matrix = stepsMatrix(runs, 1);

    expect(matrix.keys).toEqual(["up"]);
    expect(matrix.cells).toEqual([[1]]);
    expect([matrix.shown, matrix.total]).toEqual([1, 3]);
  });
});

describe("neighbours and loops", () => {
  it("lists the steps after and before a command and the loops, most first", () => {
    const runs = session(1, [
      { command: "down" },
      {},
      { command: "status" },
      {},
      { command: "status" },
      { command: "status" },
    ]);

    expect(neighbours(runs, "up")).toEqual({
      after: [["status", 2]],
      before: [
        ["down", 1],
        ["status", 1],
      ],
    });
    expect(loops(runs)).toEqual([["status", 1]]);
  });
});

describe("flagRows", () => {
  it("counts each flag a command's runs passed and notes required, never passed, overridden and global flags", () => {
    const runs = session(1, [
      { flags: ["--need", "--lines", "--json"] },
      { flags: ["--need", "--lines"] },
      { flags: ["--need"] },
      { flags: ["--need"] },
      { command: "down", flags: ["--follow"] },
    ]);
    const declared = [
      flag("--need", { required: true }),
      flag("--lines", { defaults: ["50"] }),
      flag("--follow"),
      flag("--json", { crumbkit_flag: "--json", global: true }),
      flag("--full", { crumbkit_flag: "--full", global: true }),
      flag("--dry-run", { crumbkit_flag: "--dry-run" }),
      { ...flag("<PROCESS>"), kind: "positional" as const },
    ];

    const rows = flagRows(declared, runs, "up");

    expect(rows).toEqual({
      runs: 4,
      flags: [
        { name: "--need", runs: 4, note: "required", tone: "plain", global: false },
        {
          name: "--lines",
          runs: 2,
          note: "overrides the default of 50 on 50% of runs",
          tone: "muted",
          global: false,
        },
        { name: "--follow", runs: 0, note: "never passed", tone: "bad", global: false },
        // crumbkit's --dry-run belongs to the command, so it is one of its own.
        { name: "--dry-run", runs: 0, note: "never passed", tone: "bad", global: false },
        { name: "--json", runs: 1, note: "global flag", tone: "muted", global: true },
      ],
    });
  });

  it("notes a flag that most runs pass", () => {
    const runs = session(1, [
      { flags: ["--dry-run"] },
      { flags: ["--dry-run"] },
      {},
      { flags: ["--dry-run"] },
    ]);

    expect(flagRows([flag("--dry-run")], runs, "up").flags[0]?.note).toBe("on 3 of 4 runs");
  });
});

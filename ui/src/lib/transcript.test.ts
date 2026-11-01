import { describe, expect, it } from "vitest";
import { type Run, transcript } from "./transcript";

const request = { path: ["books", "show"], values: { "<ID>": { many: ["2"] } } };

describe("transcript", () => {
  it("keeps the old runs when a new one starts", () => {
    let runs: Run[] = [];
    runs = transcript(runs, { type: "add", number: 1, request, line: "shelf books show 2" });
    runs = transcript(runs, { type: "add", number: 2, request, line: "shelf books show 2" });

    expect(runs.map((run) => [run.number, run.state.kind])).toEqual([
      [1, "starting"],
      [2, "starting"],
    ]);
  });

  it("follows a run from its start through its lines to its end", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line" });
    const events = [
      {
        event: "started",
        run: "r1",
        line: "shelf books show 2",
        started_at: "2026-11-02T09:00:00Z",
      },
      { event: "line", text: '{"schema":"s","data":[]}' },
      { event: "ended", outcome: { kind: "result" }, exit: 0, duration_ms: 412 },
    ] as const;
    for (const event of events) {
      runs = transcript(runs, { type: "event", number: 1, event });
    }

    expect(runs[0]).toEqual({
      number: 1,
      request,
      line: "shelf books show 2",
      state: {
        kind: "ended",
        startedAt: "2026-11-02T09:00:00Z",
        lines: ['{"schema":"s","data":[]}'],
        exit: 0,
        durationMs: 412,
        outcome: { kind: "result" },
      },
    });
  });
});

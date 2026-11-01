import { describe, expect, it } from "vitest";
import { type Run, transcript } from "./transcript";

const request = { path: ["books", "show"], values: { "<ID>": ["2"] } };

describe("transcript", () => {
  it("keeps the old runs when a new one starts", () => {
    let runs: Run[] = [];
    runs = transcript(runs, {
      type: "add",
      number: 1,
      request,
      line: "shelf books show 2",
      long: false,
    });
    runs = transcript(runs, {
      type: "add",
      number: 2,
      request,
      line: "shelf books show 2",
      long: false,
    });

    expect(runs.map((run) => [run.number, run.state.kind])).toEqual([
      [1, "starting"],
      [2, "starting"],
    ]);
  });

  it("follows a run from its start through its lines to its end, in one frame's batch of events", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line", long: false });
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
    runs = transcript(runs, { type: "events", number: 1, events: [...events] });

    expect(runs[0]).toEqual({
      number: 1,
      request,
      line: "shelf books show 2",
      long: false,
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

  it("ignores the keepalive lines of a quiet run", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line", long: false });
    const started = {
      event: "started",
      run: "r1",
      line: "shelf books show 2",
      started_at: "2026-11-02T09:00:00Z",
    } as const;
    runs = transcript(runs, { type: "events", number: 1, events: [started] });
    const before = runs;
    runs = transcript(runs, { type: "events", number: 1, events: [{ event: "alive" }] });

    expect(runs).toEqual(before);
  });

  it("shows a stream that closes before its ended line as cut off, with the lines so far", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line", long: false });
    const started = {
      event: "started",
      run: "r1",
      line: "shelf books show 2",
      started_at: "2026-11-02T09:00:00Z",
    } as const;
    runs = transcript(runs, { type: "events", number: 1, events: [started] });
    runs = transcript(runs, { type: "events", number: 1, events: [{ event: "line", text: "{}" }] });
    runs = transcript(runs, { type: "closed", number: 1 });

    expect(runs[0]?.state).toEqual({
      kind: "cut",
      startedAt: "2026-11-02T09:00:00Z",
      lines: ["{}"],
      words: null,
    });
  });

  it("keeps an ended run as it is when its stream closes", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line", long: false });
    const ended = { event: "ended", outcome: { kind: "result" }, exit: 0, duration_ms: 1 } as const;
    runs = transcript(runs, { type: "events", number: 1, events: [ended] });
    const before = runs;
    runs = transcript(runs, { type: "closed", number: 1 });

    expect(runs).toEqual(before);
  });

  it("keeps the lines of a stream that breaks after it started, as a cut-off run with the error", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line", long: false });
    const started = {
      event: "started",
      run: "r1",
      line: "shelf books show 2",
      started_at: "2026-11-02T09:00:00Z",
    } as const;
    runs = transcript(runs, { type: "events", number: 1, events: [started] });
    runs = transcript(runs, { type: "events", number: 1, events: [{ event: "line", text: "{}" }] });
    runs = transcript(runs, { type: "broken", number: 1, words: "The server did not answer." });

    expect(runs[0]?.state).toEqual({
      kind: "cut",
      startedAt: "2026-11-02T09:00:00Z",
      lines: ["{}"],
      words: "The server did not answer.",
    });
  });
});

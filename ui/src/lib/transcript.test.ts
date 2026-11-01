import { describe, expect, it } from "vitest";
import { type Run, transcript } from "./transcript";

const request = { path: ["books", "show"], values: { "<ID>": ["2"] } };

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

  it("ignores the keepalive lines of a quiet run", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line" });
    const started = {
      event: "started",
      run: "r1",
      line: "shelf books show 2",
      started_at: "2026-11-02T09:00:00Z",
    } as const;
    runs = transcript(runs, { type: "event", number: 1, event: started });
    const before = runs;
    runs = transcript(runs, { type: "event", number: 1, event: { event: "alive" } });

    expect(runs).toEqual(before);
  });

  it("shows a stream that closes before its ended line as cut off, with the lines so far", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line" });
    const started = {
      event: "started",
      run: "r1",
      line: "shelf books show 2",
      started_at: "2026-11-02T09:00:00Z",
    } as const;
    runs = transcript(runs, { type: "event", number: 1, event: started });
    runs = transcript(runs, { type: "event", number: 1, event: { event: "line", text: "{}" } });
    runs = transcript(runs, { type: "closed", number: 1 });

    expect(runs[0]?.state).toEqual({
      kind: "cut",
      startedAt: "2026-11-02T09:00:00Z",
      lines: ["{}"],
      words: null,
    });
  });

  it("keeps an ended run as it is when its stream closes", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line" });
    const ended = { event: "ended", outcome: { kind: "result" }, exit: 0, duration_ms: 1 } as const;
    runs = transcript(runs, { type: "event", number: 1, event: ended });
    const before = runs;
    runs = transcript(runs, { type: "closed", number: 1 });

    expect(runs).toEqual(before);
  });

  it("keeps the lines of a stream that breaks after it started, as a cut-off run with the error", () => {
    let runs = transcript([], { type: "add", number: 1, request, line: "page line" });
    const started = {
      event: "started",
      run: "r1",
      line: "shelf books show 2",
      started_at: "2026-11-02T09:00:00Z",
    } as const;
    runs = transcript(runs, { type: "event", number: 1, event: started });
    runs = transcript(runs, { type: "event", number: 1, event: { event: "line", text: "{}" } });
    runs = transcript(runs, { type: "broken", number: 1, words: "The server did not answer." });

    expect(runs[0]?.state).toEqual({
      kind: "cut",
      startedAt: "2026-11-02T09:00:00Z",
      lines: ["{}"],
      words: "The server did not answer.",
    });
  });
});

describe("a preview card", () => {
  const remove = { path: ["books", "remove"], values: { "<ID>": ["2"] } };
  const started = {
    event: "started",
    run: "r1",
    line: "shelf books remove 2",
    started_at: "2026-11-02T09:00:00Z",
    preview: true,
    confirmation: null,
  } as const;

  function previewed(): Run[] {
    return transcript([], {
      type: "preview",
      number: 1,
      request: remove,
      line: "shelf books remove 2",
      basis: null,
    });
  }

  it("holds the confirmation id that its dry run ended with", () => {
    let runs = previewed();
    runs = transcript(runs, { type: "event", number: 1, event: started });
    const ended = {
      event: "ended",
      outcome: { kind: "result" },
      exit: 0,
      duration_ms: 4,
      confirmation: "c1",
    } as const;
    runs = transcript(runs, { type: "event", number: 1, event: ended });

    expect(runs[0]?.ask).toEqual({ basis: null, confirmation: "c1", answer: "open" });
    expect(runs[0]?.state.kind).toBe("ended");
  });

  it("is complete at the started line of a command without a dry run", () => {
    let runs = previewed();
    const issued = { ...started, run: null, preview: false, confirmation: "c1" } as const;
    runs = transcript(runs, { type: "event", number: 1, event: issued });
    runs = transcript(runs, { type: "closed", number: 1 });

    expect(runs[0]?.state).toEqual({ kind: "issued" });
    expect(runs[0]?.ask?.confirmation).toBe("c1");
  });

  it("starts again as a run when the person confirms it", () => {
    let runs = previewed();
    runs = transcript(runs, { type: "answer", number: 1, answer: "confirmed" });

    expect(runs[0]?.ask?.answer).toBe("confirmed");
    expect(runs[0]?.state).toEqual({ kind: "starting" });
  });

  it("keeps its rows when the person cancels it", () => {
    let runs = previewed();
    runs = transcript(runs, { type: "event", number: 1, event: started });
    const before = runs[0]?.state;
    runs = transcript(runs, { type: "answer", number: 1, answer: "declined" });

    expect(runs[0]?.ask?.answer).toBe("declined");
    expect(runs[0]?.state).toBe(before);
  });

  it("takes only the first answer", () => {
    let runs = previewed();
    runs = transcript(runs, { type: "answer", number: 1, answer: "declined" });
    runs = transcript(runs, { type: "answer", number: 1, answer: "confirmed" });

    expect(runs[0]?.ask?.answer).toBe("declined");
  });
});

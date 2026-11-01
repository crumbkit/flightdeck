import { describe, expect, it } from "vitest";
import { parseEvent } from "./runs";

describe("parseEvent", () => {
  it("reads a preview's started and ended lines, whose confirmation fields the page does not use", () => {
    const started = parseEvent(
      '{"event":"started","run":null,"line":"shelf books remove 2","started_at":"2026-11-02T09:00:00Z","preview":false,"confirmation":"c1"}',
    );
    const ended = parseEvent(
      '{"event":"ended","outcome":{"kind":"cancelled"},"exit":130,"duration_ms":5,"confirmation":null}',
    );

    expect(started).toEqual({
      event: "started",
      run: null,
      line: "shelf books remove 2",
      started_at: "2026-11-02T09:00:00Z",
    });
    expect(ended).toEqual({
      event: "ended",
      outcome: { kind: "cancelled" },
      exit: 130,
      duration_ms: 5,
    });
  });

  it("reads a keepalive line", () => {
    expect(parseEvent('{"event":"alive"}')).toEqual({ event: "alive" });
  });
});

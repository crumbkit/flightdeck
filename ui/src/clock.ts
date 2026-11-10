import { useEffect, useRef, useState } from "react";

// The clock's owner (BASE-7): the only module that reads the time or
// schedules work for later. A running card's elapsed time is the one
// figure the page measures itself. A run's start time and duration come
// from the server.

/**
 * Returns the time in milliseconds since the epoch, read again every
 * `intervalMs` while `active` is true. The server runs on the same machine
 * as the browser, so the time compares with a run's `started_at`.
 */
export function useNow(intervalMs: number, active: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) {
      return;
    }
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(timer);
  }, [intervalMs, active]);
  return now;
}

/** Returns the time in milliseconds since the epoch when the component mounted. */
export function useMountTime(): number {
  const [now] = useState(() => Date.now());
  return now;
}

/**
 * Returns `value` as it was at the last tick of an `intervalMs` timer while
 * `active` is true, so a live region changes at most once per interval,
 * and `value` itself once `active` is false.
 */
export function useSampled<T>(value: T, intervalMs: number, active: boolean): T {
  const latest = useRef(value);
  const [sampled, setSampled] = useState(value);
  useEffect(() => {
    latest.current = value;
  });
  useEffect(() => {
    if (!active) {
      return;
    }
    const timer = setInterval(() => setSampled(latest.current), intervalMs);
    return () => clearInterval(timer);
  }, [intervalMs, active]);
  return active ? sampled : value;
}

/** A queue whose items reach their consumer once per animation frame. */
export type FrameQueue<T> = {
  push: (item: T) => void;
  /** Delivers the waiting items at once, such as when a stream ends. */
  flush: () => void;
};

/**
 * Returns a queue that delivers the items pushed during one animation
 * frame to `deliver` together, in order, so a fast stream draws once per
 * frame rather than once per line.
 */
export function frameQueue<T>(deliver: (items: T[]) => void): FrameQueue<T> {
  let waiting: T[] = [];
  let frame: number | null = null;
  const flush = () => {
    if (frame !== null) {
      cancelAnimationFrame(frame);
      frame = null;
    }
    if (waiting.length > 0) {
      const items = waiting;
      waiting = [];
      deliver(items);
    }
  };
  return {
    push: (item) => {
      waiting.push(item);
      frame ??= requestAnimationFrame(() => {
        frame = null;
        flush();
      });
    },
    flush,
  };
}

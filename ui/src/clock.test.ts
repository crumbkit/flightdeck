import { afterEach, describe, expect, it, vi } from "vitest";
import { frameQueue } from "./clock";

afterEach(() => {
  vi.unstubAllGlobals();
});

/** Stubs the browser's frames, and returns a function that draws the next one. */
function stubFrames(): () => void {
  let pending: FrameRequestCallback | null = null;
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
    pending = callback;
    return 1;
  });
  vi.stubGlobal("cancelAnimationFrame", () => {
    pending = null;
  });
  return () => {
    const callback = pending;
    pending = null;
    callback?.(0);
  };
}

describe("frameQueue", () => {
  it("delivers the items pushed during one frame together, in order", () => {
    const draw = stubFrames();
    const delivered: number[][] = [];
    const queue = frameQueue<number>((items) => delivered.push(items));

    queue.push(1);
    queue.push(2);
    queue.push(3);
    expect(delivered).toEqual([]);
    draw();

    expect(delivered).toEqual([[1, 2, 3]]);
  });

  it("delivers what is left at once on flush, and nothing at the next frame", () => {
    const draw = stubFrames();
    const delivered: number[][] = [];
    const queue = frameQueue<number>((items) => delivered.push(items));

    queue.push(1);
    queue.flush();
    draw();
    queue.flush();

    expect(delivered).toEqual([[1]]);
  });
});

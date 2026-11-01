import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Transcript } from "./Transcript";

/** Gives the scroller a layout, which jsdom does not compute. */
function layOut(element: HTMLElement, scrollTop: number) {
  Object.defineProperty(element, "scrollHeight", { configurable: true, value: 1000 });
  Object.defineProperty(element, "clientHeight", { configurable: true, value: 100 });
  element.scrollTop = scrollTop;
}

describe("Transcript", () => {
  it("announces the New rows button politely when rows arrive below a reader who scrolled up", () => {
    const { rerender } = render(
      <Transcript size={1}>
        <p>first</p>
      </Transcript>,
    );
    const status = screen.getByRole("status");
    const scroller = status.previousElementSibling;
    if (!(scroller instanceof HTMLElement)) {
      throw new Error("the transcript has no scroller before its status");
    }
    layOut(scroller, 0);
    fireEvent.scroll(scroller);

    rerender(
      <Transcript size={2}>
        <p>first</p>
        <p>second</p>
      </Transcript>,
    );

    expect(status).toHaveAttribute("aria-live", "polite");
    expect(status).toContainElement(screen.getByRole("button", { name: "New rows" }));
  });
});

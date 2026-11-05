import { fireEvent, render, screen } from "@testing-library/react";
import { StrictMode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Transcript } from "./Transcript";

/** Gives the scroller a layout, which jsdom does not compute. */
function layOut(element: HTMLElement, scrollTop: number) {
  Object.defineProperty(element, "scrollHeight", { configurable: true, value: 1000 });
  Object.defineProperty(element, "clientHeight", { configurable: true, value: 100 });
  Object.defineProperty(element, "scrollTop", { configurable: true, writable: true, value: 0 });
  element.scrollTop = scrollTop;
}

/** The scroller, which sits just before the transcript's status region. */
function scrollerOf(): HTMLElement {
  const scroller = screen.getByRole("status").previousElementSibling;
  if (!(scroller instanceof HTMLElement)) {
    throw new Error("the transcript has no scroller before its status");
  }
  return scroller;
}

afterEach(() => {
  vi.restoreAllMocks();
  Reflect.deleteProperty(HTMLElement.prototype, "scrollTop");
});

describe("Transcript", () => {
  it("does not scroll when it first appears, even when React runs its effects twice", () => {
    const set = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "scrollTop", {
      configurable: true,
      get: () => 0,
      set,
    });

    render(
      <StrictMode>
        <Transcript content="" latest={null}>
          <p>welcome</p>
        </Transcript>
      </StrictMode>,
    );

    expect(set).not.toHaveBeenCalled();
  });

  it("announces the New rows button politely when rows arrive below a reader who scrolled up", () => {
    const { rerender } = render(
      <Transcript content="1" latest="run-1">
        <p>first</p>
      </Transcript>,
    );
    const status = screen.getByRole("status");
    const scroller = scrollerOf();
    layOut(scroller, 0);
    fireEvent.scroll(scroller);

    rerender(
      <Transcript content="2" latest="run-1">
        <p>first</p>
        <p>second</p>
      </Transcript>,
    );

    expect(status).toHaveAttribute("aria-live", "polite");
    expect(status).toContainElement(screen.getByRole("button", { name: "New rows" }));
  });

  it("shows the New rows button when the newest card grows past the bottom of the view", () => {
    const { rerender } = render(
      <Transcript content="1" latest="run-1">
        <p id="run-1">first</p>
      </Transcript>,
    );
    const scroller = scrollerOf();
    layOut(scroller, 0);
    const card = document.getElementById("run-1");
    if (card === null) {
      throw new Error("the card is not in the document");
    }
    // The card starts 50px down the content, so the view stops there and
    // the card's later rows are out of sight below it.
    card.getBoundingClientRect = () => DOMRect.fromRect({ x: 0, y: 50, width: 100, height: 900 });

    rerender(
      <Transcript content="2" latest="run-1">
        <p id="run-1">first and many more rows</p>
      </Transcript>,
    );

    expect(scroller.scrollTop).toBe(50);
    expect(screen.getByRole("button", { name: "New rows" })).toBeInTheDocument();
  });

  it("takes the New rows button away when a new card arrives, because the view moves to the card", () => {
    const { rerender } = render(
      <Transcript content="1" latest="run-1">
        <p>first</p>
      </Transcript>,
    );
    const scroller = scrollerOf();
    layOut(scroller, 0);
    fireEvent.scroll(scroller);
    rerender(
      <Transcript content="2" latest="run-1">
        <p>first</p>
        <p>second</p>
      </Transcript>,
    );
    expect(screen.getByRole("button", { name: "New rows" })).toBeInTheDocument();
    // The new card is short and sits at the end of the content.
    const box = Element.prototype.getBoundingClientRect;
    vi.spyOn(Element.prototype, "getBoundingClientRect").mockImplementation(function (
      this: Element,
    ) {
      return this.id === "run-2"
        ? DOMRect.fromRect({ x: 0, y: 980, width: 100, height: 20 })
        : box.call(this);
    });

    rerender(
      <Transcript content="3" latest="run-2">
        <p>first</p>
        <p>second</p>
        <p id="run-2">third</p>
      </Transcript>,
    );

    expect(screen.queryByRole("button", { name: "New rows" })).not.toBeInTheDocument();
  });
});

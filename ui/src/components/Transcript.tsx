import { ArrowDown } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";

type Props = {
  /** Changes whenever a card changes what it draws, such as `drawnKey` of the runs. */
  content: string;
  /** The element id of the newest card, or null before the first run. */
  latest: string | null;
  children: ReactNode;
};

/** How close to the bottom, in pixels, still counts as at the bottom. */
const slack = 24;

/** Scrolls `element` to `top`, and returns where the browser put it. */
function scrollTo(element: HTMLElement, top: number): number {
  element.scrollTop = top;
  return element.scrollTop;
}

/**
 * How the view moves when the content grows. `newest` keeps the newest card
 * in view: the view goes to the bottom, but never so far that the card's
 * top leaves it. `bottom` follows the end of the content. `still` leaves the
 * view where the reader put it.
 */
type Follow = "newest" | "bottom" | "still";

/**
 * The scrolling transcript. It opens where its content starts. A new card
 * scrolls into view with its top showing, and when its rows run past the
 * bottom of the view, a "New rows ↓" button returns to the bottom. While
 * the reader stays at the bottom, the view follows new rows. When they have
 * scrolled up, new content does not move the view, and the button shows.
 */
export function Transcript({ content, latest, children }: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const follow = useRef<Follow>("newest");
  // The content and newest card the view last answered. The effect acts
  // only when one of them changes, so the first render, and React's second
  // run of its effects in StrictMode, leave the page at its start.
  const drawn = useRef(content);
  const shown = useRef<string | null>(latest);
  // The position this component last scrolled to, so its own scroll event
  // is not read as the reader's.
  const placed = useRef<number | null>(null);
  const [behind, setBehind] = useState(false);

  // The effect follows the browser's scroll position, which lives outside
  // React, each time a card changes what it draws or a new card arrives.
  useEffect(() => {
    const element = scroller.current;
    if (element === null || (content === drawn.current && latest === shown.current)) {
      return;
    }
    drawn.current = content;
    if (latest !== shown.current) {
      shown.current = latest;
      follow.current = "newest";
      setBehind(false);
    }
    const bottom = element.scrollHeight - element.clientHeight;
    if (follow.current === "bottom") {
      placed.current = scrollTo(element, bottom);
    } else if (follow.current === "newest") {
      const card = latest === null ? null : document.getElementById(latest);
      const top =
        card === null || !element.contains(card)
          ? bottom
          : card.getBoundingClientRect().top -
            element.getBoundingClientRect().top +
            element.scrollTop -
            Number.parseFloat(getComputedStyle(element).paddingTop || "0");
      const target = Math.max(0, Math.min(bottom, top));
      placed.current = scrollTo(element, target);
      // The card's top is in view, and its later rows are below it.
      if (bottom - target > slack) {
        setBehind(true);
      }
    } else {
      setBehind(true);
    }
  }, [content, latest]);

  const onScroll = () => {
    const element = scroller.current;
    if (element === null) {
      return;
    }
    if (placed.current !== null && Math.abs(element.scrollTop - placed.current) < 1) {
      return;
    }
    placed.current = null;
    const atBottom = element.scrollHeight - element.scrollTop - element.clientHeight <= slack;
    follow.current = atBottom ? "bottom" : "still";
    if (atBottom) {
      setBehind(false);
    }
  };

  const toBottom = () => {
    const element = scroller.current;
    if (element !== null) {
      placed.current = scrollTo(element, element.scrollHeight);
    }
    follow.current = "bottom";
    setBehind(false);
  };

  return (
    <div className="relative min-h-0 flex-1">
      <div ref={scroller} onScroll={onScroll} className="h-full overflow-y-auto px-4 pt-3 pb-2">
        {children}
      </div>
      {/* The live region is always present, so a screen reader announces
          the button when it appears. */}
      <div role="status" aria-live="polite" className="absolute bottom-3 left-1/2 -translate-x-1/2">
        {behind && (
          <button
            type="button"
            onClick={toBottom}
            className="bg-card text-primary hover:bg-accent lift-menu transition-tint active:bg-muted rounded-pill inline-flex min-h-8 items-center gap-1 border px-3 py-1 text-xs"
          >
            New rows
            <ArrowDown aria-hidden="true" className="size-3.5" />
          </button>
        )}
      </div>
    </div>
  );
}

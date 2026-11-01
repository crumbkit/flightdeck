import { ArrowDown } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";

type Props = {
  /** Changes whenever the transcript's content grows, such as a count of its lines. */
  size: number;
  children: ReactNode;
};

/** How close to the bottom, in pixels, still counts as at the bottom. */
const slack = 24;

/**
 * The scrolling transcript. While the person is at the bottom, it follows
 * the newest card. When they have scrolled up, new content does not move
 * the view, and a "New rows ↓" button returns to the bottom.
 */
export function Transcript({ size, children }: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const atBottom = useRef(true);
  const [behind, setBehind] = useState(false);

  // The effect follows the browser's scroll position, which lives outside
  // React, each time the content grows.
  // biome-ignore lint/correctness/useExhaustiveDependencies: size is the trigger, read through the DOM
  useEffect(() => {
    const element = scroller.current;
    if (element === null) {
      return;
    }
    if (atBottom.current) {
      element.scrollTop = element.scrollHeight;
    } else {
      setBehind(true);
    }
  }, [size]);

  const onScroll = () => {
    const element = scroller.current;
    if (element === null) {
      return;
    }
    atBottom.current = element.scrollHeight - element.scrollTop - element.clientHeight <= slack;
    if (atBottom.current) {
      setBehind(false);
    }
  };

  const toBottom = () => {
    const element = scroller.current;
    if (element !== null) {
      element.scrollTop = element.scrollHeight;
    }
    atBottom.current = true;
    setBehind(false);
  };

  return (
    <div className="relative min-h-0 flex-1">
      <div ref={scroller} onScroll={onScroll} className="h-full overflow-y-auto px-4 pt-3 pb-2">
        {children}
      </div>
      {behind && (
        <button
          type="button"
          onClick={toBottom}
          className="bg-card text-primary absolute bottom-3 left-1/2 inline-flex -translate-x-1/2 items-center gap-1 rounded-full border px-3 py-1 text-xs shadow-md"
        >
          New rows
          <ArrowDown aria-hidden="true" className="size-3.5" />
        </button>
      )}
    </div>
  );
}

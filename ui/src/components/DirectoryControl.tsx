import { ChevronDown, Folder } from "lucide-react";
import { useEffect, useRef, useState } from "react";

type Props = {
  /** The working directory, or null for the directory flightdeck started in. */
  directory: string | null;
  onChange: (directory: string) => void;
};

/** The words for runs that start where flightdeck did. */
const whereStarted = "where flightdeck started";

/**
 * The top bar's working directory. Each run starts in the directory shown
 * when Run is pressed. Pressing the button opens a box that takes a typed
 * path: Enter or leaving the box keeps it, and Escape or a blank box does
 * not.
 *
 * Soft limit: the box takes any text, and the server checks it when a run
 * starts, so a mistyped path shows as a refused run. It misses a check while
 * the person types and a way to browse. The path picker's dialog replaces
 * the box.
 */
export function DirectoryControl({ directory, onChange }: Props) {
  const [typing, setTyping] = useState<string | null>(null);
  const button = useRef<HTMLButtonElement>(null);
  const box = useRef<HTMLInputElement>(null);
  // A key that closes the box gives focus back to the button. Leaving the
  // box by Tab or a click must leave focus where it went.
  const refocus = useRef(false);
  const open = typing !== null;

  useEffect(() => {
    if (open) {
      box.current?.select();
      box.current?.focus();
    } else if (refocus.current) {
      refocus.current = false;
      button.current?.focus();
    }
  }, [open]);

  const keep = () => {
    if (typing !== null && typing !== "" && typing !== directory) {
      onChange(typing);
    }
    setTyping(null);
  };

  if (typing !== null) {
    return (
      <input
        ref={box}
        type="text"
        aria-label="Working directory"
        value={typing}
        spellCheck={false}
        autoCapitalize="off"
        autoCorrect="off"
        onChange={(event) => setTyping(event.target.value)}
        onBlur={keep}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            refocus.current = true;
            keep();
          } else if (event.key === "Escape") {
            refocus.current = true;
            setTyping(null);
          }
        }}
        className="border-input bg-background min-h-11 w-full min-w-0 rounded-md border px-2 font-mono text-xs md:min-h-8 md:max-w-md"
      />
    );
  }
  const shown = directory ?? whereStarted;
  return (
    <button
      ref={button}
      type="button"
      aria-label={`Working directory ${shown}. Change`}
      title={shown}
      onClick={() => setTyping(directory ?? "")}
      className="bg-card hover:bg-accent transition-tint inline-flex min-h-11 w-full min-w-0 items-center gap-1.5 rounded-md border px-2 font-mono text-xs md:min-h-8 md:w-auto md:max-w-md"
    >
      <Folder aria-hidden="true" className="size-3.5 shrink-0" />
      <span className="min-w-0 flex-1 truncate text-left">{shown}</span>
      <ChevronDown aria-hidden="true" className="size-3.5 shrink-0" />
    </button>
  );
}

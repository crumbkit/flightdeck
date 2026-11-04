import { Search } from "lucide-react";
import { type KeyboardEvent, useId, useState } from "react";
import type { Command, Example, Tool } from "@/api/model";
import { search } from "@/lib/palette";
import { effectOf } from "@/lib/rail";
import { EffectBadge } from "./EffectBadge";
import { CommandPathText } from "./Rail";

type Props = {
  tool: Tool;
  onOpen: (command: Command, example: Example | null) => void;
  onClose: () => void;
};

/**
 * The ⌘K palette: a modal dialog that finds a command by its path, its
 * about text or an example's caption. ↑ and ↓ move, ↩ opens the command in
 * the composer, and Esc closes.
 */
export function Palette({ tool, onOpen, onClose }: Props) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const matches = search(tool, query);
  const listId = useId();
  const optionId = (index: number) => `${listId}-${index}`;

  const choose = (index: number) => {
    const match = matches[index];
    if (match !== undefined) {
      onOpen(match.command, match.example);
    }
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive(Math.min(active + 1, matches.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive(Math.max(active - 1, 0));
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(active);
    }
  };

  return (
    <dialog
      // The dialog opens as a modal as soon as React attaches it.
      ref={(element) => {
        if (element !== null && !element.open) {
          element.showModal();
        }
      }}
      aria-label="Find a command"
      onClose={onClose}
      className="bg-card text-foreground backdrop:bg-foreground/30 mx-auto mt-16 w-full max-w-xl rounded-xl border p-0 shadow-lg"
    >
      <div className="flex items-center gap-2.5 border-b px-3.5 py-2.5">
        <Search aria-hidden="true" className="text-faint-foreground size-4" />
        <input
          role="combobox"
          aria-expanded="true"
          aria-controls={listId}
          aria-activedescendant={matches.length > 0 ? optionId(active) : undefined}
          aria-label="Command"
          placeholder="Find a command"
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
          }}
          onKeyDown={onKeyDown}
          className="flex-1 bg-transparent font-mono text-base outline-none"
        />
      </div>
      <div
        id={listId}
        role="listbox"
        aria-label="Commands"
        className="m-0 max-h-80 overflow-y-auto p-1.5"
      >
        {matches.map((match, index) => (
          <div
            key={match.command.path.join(" ")}
            id={optionId(index)}
            role="option"
            tabIndex={-1}
            aria-selected={index === active}
            onClick={() => choose(index)}
            onKeyDown={onKeyDown}
            onMouseMove={() => setActive(index)}
            className="aria-selected:bg-accent hover:bg-muted flex cursor-pointer items-center gap-2.5 rounded-lg px-2.5 py-1.5"
          >
            <span className="shrink-0">
              <CommandPathText command={match.command} />
            </span>
            <span className="text-muted-foreground min-w-0 flex-1 truncate text-xs">
              {match.example?.about ?? match.command.about}
            </span>
            <EffectBadge effect={effectOf(match.command)} />
          </div>
        ))}
        {matches.length === 0 && (
          <p className="text-muted-foreground m-0 px-2.5 py-1.5">No command matches.</p>
        )}
      </div>
      <p className="text-faint-foreground m-0 flex gap-3.5 border-t px-3.5 py-2 text-xs">
        <span>↑ ↓ move</span>
        <span>↩ open</span>
        <span>Esc close</span>
      </p>
    </dialog>
  );
}

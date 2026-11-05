import { X } from "lucide-react";
import { type KeyboardEvent, type ReactNode, useId, useState } from "react";

type Props = {
  label: string;
  name: ReactNode;
  required: ReactNode;
  items: string[];
  error: ReactNode;
  errorId: string | undefined;
  onItems: (items: string[]) => void;
};

/**
 * Returns each item with a key that stays the same while the items before it
 * do, such as "2#0" for the first 2, so a value may appear twice.
 */
function keyed(items: string[]): { key: string; item: string; index: number }[] {
  const seen = new Map<string, number>();
  return items.map((item, index) => {
    const count = seen.get(item) ?? 0;
    seen.set(item, count + 1);
    return { key: `${item}#${count}`, item, index };
  });
}

/**
 * The input of an argument that repeats, such as the ids of `books show`.
 * Each value is a chip with a remove button. A space, a comma, ↩ or leaving
 * the field adds what was typed, and Backspace in the empty field removes
 * the last chip.
 */
export function ChipsInput({ label, name, required, items, error, errorId, onItems }: Props) {
  const [text, setText] = useState("");
  const inputId = useId();
  const commit = () => {
    const added = text.split(/[\s,]+/).filter((word) => word !== "");
    if (added.length > 0) {
      onItems([...items, ...added]);
    }
    setText("");
  };
  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (
      (event.key === "Enter" && !event.metaKey && !event.ctrlKey) ||
      event.key === "," ||
      event.key === " "
    ) {
      event.preventDefault();
      commit();
    } else if (event.key === "Backspace" && text === "" && items.length > 0) {
      onItems(items.slice(0, -1));
    }
  };
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={inputId} className="flex flex-wrap items-baseline gap-2 text-xs font-medium">
        {label} {name} {required}
      </label>
      <div className="border-input bg-background focus-within:border-ring transition-tint flex min-h-8 max-w-xl flex-wrap items-center gap-1.5 rounded-md border px-2 py-1">
        {keyed(items).map(({ key, item, index }) => (
          <span
            key={key}
            className="bg-accent inline-flex items-center gap-1 rounded py-0.5 pr-1 pl-2 font-mono"
          >
            {item}
            <button
              type="button"
              aria-label={`Remove ${item}`}
              onClick={() => onItems(items.filter((_, other) => other !== index))}
              className="text-faint-foreground"
            >
              <X aria-hidden="true" className="size-3" />
            </button>
          </span>
        ))}
        <input
          id={inputId}
          type="text"
          value={text}
          aria-invalid={error === null ? undefined : true}
          aria-describedby={errorId}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={onKeyDown}
          onBlur={commit}
          className="min-w-16 flex-1 bg-transparent py-0.5 font-mono outline-none"
        />
      </div>
      {error}
    </div>
  );
}

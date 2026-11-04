import { useQuery } from "@tanstack/react-query";
import { choicesQuery } from "@/api/choices";
import type { Link, Tool } from "@/api/model";
import type { Value } from "@/api/runs";
import { chooserFooter } from "@/lib/links";

type Props = {
  tool: Tool;
  link: Link;
  /** True when the argument repeats, so each checked choice adds a value. */
  many: boolean;
  value: Value | undefined;
  onValue: (value: Value | null) => void;
};

/** Returns a label cell as text, or null for a cell that has no value. */
function cellText(cell: unknown): string | null {
  if (cell === null || cell === undefined) {
    return null;
  }
  return typeof cell === "string" ? cell : JSON.stringify(cell);
}

/** Returns the values that a field holds, as a list. */
function valuesOf(value: Value | undefined): string[] {
  if (typeof value === "string") {
    return [value];
  }
  return Array.isArray(value) ? value : [];
}

/**
 * The IDs chooser under an argument that an inferred link fills: the ids
 * of the source's rows, each labelled by the row's other compact columns.
 * Checking a choice adds its id to the field, and the field still takes a
 * typed value. The server runs the source when the chooser first shows,
 * and the page keeps the choices until it reloads.
 */
export function IdChooser({ tool, link, many, value, onValue }: Props) {
  const choices = useQuery(choicesQuery(link));
  const values = valuesOf(value);

  const toggle = (id: string, on: boolean) => {
    if (!many) {
      onValue(on ? id : null);
      return;
    }
    const next = on ? [...values, id] : values.filter((each) => each !== id);
    onValue(next.length === 0 ? null : next);
  };

  return (
    <div className="flex max-w-xl flex-col gap-1 text-xs">
      {choices.isPending && <p className="text-muted-foreground m-0">Listing the choices…</p>}
      {choices.isError && (
        <p className="text-muted-foreground m-0">
          flightdeck could not list the choices. Type the ids instead.
        </p>
      )}
      {choices.isSuccess && (
        <fieldset className="flex max-h-40 flex-col overflow-y-auto rounded-md border p-1.5">
          <legend className="sr-only">Choices</legend>
          {choices.data.choices.map((choice) => (
            <label key={choice.value} className="flex min-h-6 items-center gap-2">
              <input
                type="checkbox"
                checked={values.includes(choice.value)}
                onChange={(event) => toggle(choice.value, event.target.checked)}
              />
              <span className="font-mono">{choice.value}</span>{" "}
              <span className="text-muted-foreground">
                {choice.label
                  .map(cellText)
                  .filter((text) => text !== null)
                  .join(" · ")}
              </span>
            </label>
          ))}
        </fieldset>
      )}
      {choices.isSuccess && choices.data.truncated && (
        <p className="text-muted-foreground m-0">
          Showing the first {choices.data.choices.length} choices. Type any other id.
        </p>
      )}
      <p className="text-faint-foreground m-0">{chooserFooter(tool, link)}</p>
    </div>
  );
}

import { cn } from "cn";
import { type ReactNode, useState } from "react";
import { bytesWords, grouped } from "@/lib/cost";
import { elapsedWords } from "@/lib/duration";
import {
  type Direction,
  type FigureKey,
  type FigureRow,
  figureOf,
  sortedRows,
} from "@/lib/figures";
import { SortHeader } from "./SortHeader";

type Props = {
  /** The tool's name, such as "shelf". */
  name: string;
  rows: FigureRow[];
  /** Opens the form of the command named, such as "books show". */
  onOpen: (command: string) => void;
};

/** Each column the table sorts by, with its name in the "Sort by" select. */
const keys: [FigureKey, string][] = [
  ["runs", "runs"],
  ["failed", "failed"],
  ["refused", "refused"],
  ["help_before", "help before"],
  ["p50_ms", "wall time, median"],
  ["p95_ms", "wall time, p95"],
  ["p50_out_bytes", "output, median"],
  ["p95_out_bytes", "output, p95"],
  ["command", "command"],
];

/** Returns the direction a column sorts in when first chosen: names A to Z, figures most first. */
function firstDirection(key: FigureKey): Direction {
  return key === "command" ? "ascending" : "descending";
}

/** A figure of the table, or a dash when the command has none. */
function Value({
  value,
  words = grouped,
}: {
  value: number | null;
  words?: (n: number) => string;
}) {
  return value === null ? <span className="text-faint-foreground">–</span> : words(value);
}

/**
 * The figures tab: every command's runs, failures, refusals, help read
 * before running, wall time and output, from the tool's `self figures
 * list`. A column header sorts the table, and a command's name opens its
 * form. Below the md width the table becomes one list row per command,
 * with a "Sort by" select in place of the headers.
 */
export function Figures({ name, rows, onOpen }: Props) {
  const [sort, setSort] = useState<{ key: FigureKey; direction: Direction }>({
    key: "runs",
    direction: "descending",
  });
  const sorted = sortedRows(rows, sort.key, sort.direction);
  const choose = (key: FigureKey) =>
    setSort(
      key === sort.key
        ? { key, direction: sort.direction === "ascending" ? "descending" : "ascending" }
        : { key, direction: firstDirection(key) },
    );
  const header = (key: FigureKey, label: string, rowSpan = 1, className = "") => (
    <SortHeader
      key={key}
      label={label}
      sort={key === sort.key ? sort.direction : null}
      onSort={() => choose(key)}
      rowSpan={rowSpan}
      className={className}
    />
  );
  const command = (path: string) => (
    <button
      type="button"
      onClick={() => onOpen(path)}
      className="text-primary min-h-6 rounded-xs text-left font-mono whitespace-nowrap hover:underline"
    >
      {path}
    </button>
  );
  return (
    <div className="px-4 pt-3 pb-4">
      <label className="text-muted-foreground mb-2 flex items-center gap-2 text-xs md:hidden">
        Sort by
        <select
          value={sort.key}
          onChange={(event) => {
            const key = keys.find(([each]) => each === event.target.value)?.[0];
            if (key !== undefined) {
              setSort({ key, direction: firstDirection(key) });
            }
          }}
          className="bg-code text-muted-foreground h-8 rounded-md border px-1.5 font-mono text-xs"
        >
          {keys.map(([key, label]) => (
            <option key={key} value={key}>
              {label}
            </option>
          ))}
        </select>
      </label>
      <div className="bg-card rounded-md border">
        <table
          aria-label="Figures of each command"
          className="hidden w-full border-collapse text-xs md:table"
        >
          <thead>
            <tr>
              {header("command", "Command", 2, "text-left")}
              {header("runs", "Runs", 2)}
              {header("failed", "Failed", 2)}
              {header("refused", "Refused", 2)}
              {header("help_before", "Help before", 2)}
              <th
                colSpan={2}
                scope="colgroup"
                className="text-muted-foreground px-2.5 pt-1.75 pb-0.5 text-center font-medium"
              >
                Wall time
              </th>
              <th
                colSpan={2}
                scope="colgroup"
                className="text-muted-foreground px-2.5 pt-1.75 pb-0.5 text-center font-medium"
              >
                Output
              </th>
            </tr>
            <tr>
              {header("p50_ms", "median")}
              {header("p95_ms", "p95")}
              {header("p50_out_bytes", "median")}
              {header("p95_out_bytes", "p95")}
            </tr>
          </thead>
          <tbody>
            {sorted.map((row) => {
              const of = (key: Exclude<FigureKey, "command">) => figureOf(row, key);
              const failed = of("failed");
              return (
                <tr key={row.command} className="hover:bg-muted border-b last:border-b-0">
                  <th scope="row" className="px-2.5 py-1.5 text-left font-normal">
                    {command(row.command)}
                  </th>
                  {row.counted ? (
                    <>
                      <Cell>
                        <Value value={of("runs")} />
                      </Cell>
                      <Cell className={cn(failed !== null && failed > 0 && "text-destructive")}>
                        <Value value={failed} />
                      </Cell>
                      <Cell>
                        <Value value={of("refused")} />
                      </Cell>
                      <Cell>
                        <Value value={of("help_before")} />
                      </Cell>
                      <Cell>
                        <Value value={of("p50_ms")} words={elapsedWords} />
                      </Cell>
                      <Cell>
                        <Value value={of("p95_ms")} words={elapsedWords} />
                      </Cell>
                      <Cell>
                        <Value value={of("p50_out_bytes")} words={bytesWords} />
                      </Cell>
                      <Cell>
                        <Value value={of("p95_out_bytes")} words={bytesWords} />
                      </Cell>
                    </>
                  ) : (
                    <td colSpan={8} className="text-muted-foreground px-2.5 py-1.5 text-left">
                      not counted
                    </td>
                  )}
                </tr>
              );
            })}
          </tbody>
        </table>
        <ul aria-label="Figures of each command" className="m-0 list-none p-0 md:hidden">
          {sorted.map((row) => {
            const of = (key: Exclude<FigureKey, "command">) => figureOf(row, key);
            const failed = of("failed");
            const runs = of("runs") ?? 0;
            return (
              <li key={row.command} className="border-b px-3 py-2.25 last:border-b-0">
                <div className="flex items-baseline justify-between gap-2 text-xs">
                  {command(row.command)}
                  {row.counted ? (
                    <span className="font-mono tabular-nums">
                      {grouped(runs)} {runs === 1 ? "run" : "runs"}
                    </span>
                  ) : (
                    <span className="text-muted-foreground">not counted</span>
                  )}
                </div>
                {row.counted && (
                  <div className="mt-1 grid grid-cols-6 gap-x-2.5 gap-y-0.5 text-xs">
                    <Pair label="failed" className="col-span-2" bad={failed !== null && failed > 0}>
                      <Value value={failed} />
                    </Pair>
                    <Pair label="refused" className="col-span-2">
                      <Value value={of("refused")} />
                    </Pair>
                    <Pair label="help before" className="col-span-2">
                      <Value value={of("help_before")} />
                    </Pair>
                    <Pair label="wall time, median · p95" className="col-span-3">
                      <Value value={of("p50_ms")} words={elapsedWords} /> ·{" "}
                      <Value value={of("p95_ms")} words={elapsedWords} />
                    </Pair>
                    <Pair label="output, median · p95" className="col-span-3">
                      <Value value={of("p50_out_bytes")} words={bytesWords} /> ·{" "}
                      <Value value={of("p95_out_bytes")} words={bytesWords} />
                    </Pair>
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      </div>
      <p className="text-faint-foreground m-0 mt-2.5 text-xs">
        From <code>{name} self figures list</code>. Choose a command to open its form.
      </p>
      {rows.some((row) => !row.counted) && (
        <p className="text-faint-foreground m-0 mt-1 text-xs">
          Not counted: flightdeck runs these commands itself to load {name}, and the figures cannot
          tell its runs from an agent's, so they leave out every run of them.
        </p>
      )}
    </div>
  );
}

function Cell({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <td
      className={cn("px-2.5 py-1.5 text-right font-mono whitespace-nowrap tabular-nums", className)}
    >
      {children}
    </td>
  );
}

/** A figure of a phone row, with its label under it. */
function Pair({
  label,
  className,
  bad = false,
  children,
}: {
  label: string;
  className: string;
  bad?: boolean;
  children: ReactNode;
}) {
  return (
    <div className={cn("flex flex-col", className)}>
      <span className={cn("font-mono tabular-nums", bad && "text-destructive")}>{children}</span>
      <span className="text-faint-foreground text-2xs">{label}</span>
    </div>
  );
}

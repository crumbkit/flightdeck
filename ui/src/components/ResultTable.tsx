import { cn } from "cn";
import { ChevronDown, ChevronRight } from "lucide-react";
import { Fragment, type ReactNode, useState } from "react";
import type { Command, Link } from "@/api/model";
import { type Entry, type ItemFailure, rowsOf } from "@/api/result";
import { cellLinkWords, isLinkColumn } from "@/lib/links";
import {
  allColumnsOf,
  columnsOf,
  footerOf,
  type PageSize,
  pageSizesOf,
  type Row,
  type View,
} from "@/lib/table";
import { CellValue } from "./CellValue";
import { RecordView } from "./RecordView";

/** A table's entry with its place among the entries, and a row's number among the rows. */
type Numbered = { position: number } & (
  | { kind: "failure"; failure: ItemFailure }
  | { kind: "row"; row: Row; index: number }
);

/** Numbers the entries in order, and the rows among them from 0. */
function numbered(entries: Entry[]): Numbered[] {
  let index = 0;
  return entries.map((entry, position) => {
    if (entry.kind === "failure") {
      return { position, ...entry };
    }
    index += 1;
    return { position, kind: "row", row: entry.row, index: index - 1 };
  });
}

type Props = {
  command: Command;
  entries: Entry[];
  listing: { total: number | null; truncated: boolean } | null;
  view: View;
  onView: (view: View) => void;
  onPageSize: (size: PageSize) => void;
  label: string;
  /** The link that each `id` cell opens, or null for none. */
  link: Link | null;
  onOpenLink: (value: string) => void;
  /** Draws a failure in the place of the row it stands for. */
  drawFailure: (failure: ItemFailure) => ReactNode;
};

const sizeLabels: Record<PageSize, string> = {
  default: "Default",
  "25": "25",
  "100": "100",
  all: "All",
};

/**
 * A result's rows as a table, with each failure of a batch or a stream as a
 * row of its own in its place. Each column shows its description under its
 * name. The toolbar adds the long columns or picks columns, which the card
 * adds to its copied line, and each row expands to its full record. A
 * listing ends with crumbkit's footer and its page sizes, each of which
 * runs the command again in a new card.
 */
export function ResultTable(props: Props) {
  const {
    command,
    entries,
    listing,
    view,
    onView,
    onPageSize,
    label,
    drawFailure,
    link,
    onOpenLink,
  } = props;
  const [open, setOpen] = useState<number | null>(null);
  const rows = rowsOf(entries);
  const columns = columnsOf(command, rows, view);
  // The columns whose figures sit right-aligned, so their heads do too.
  const numeric = new Set(
    columns
      .filter((column) => rows.some((row) => typeof row[column.name] === "number"))
      .map((column) => column.name),
  );
  const every = allColumnsOf(command, rows);
  const hasLong = every.some((column) => column.long);
  const footer = listing === null ? null : footerOf(command, rows.length, listing);
  const sizes = listing === null ? [] : pageSizesOf(command);
  const shown = columns.map((column) => column.name);

  const pick = (name: string, on: boolean) => {
    const fields = every
      .map((column) => column.name)
      .filter((each) => (each === name ? on : shown.includes(each)));
    onView({ long: view.long, fields });
  };

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-4 text-xs">
        {hasLong && (
          <label className="inline-flex min-h-8 items-center gap-1.5">
            <input
              type="checkbox"
              checked={view.long}
              disabled={view.fields !== null}
              onChange={(event) => onView({ long: event.target.checked, fields: null })}
            />
            Long columns
          </label>
        )}
        <details className="relative">
          <summary className="text-primary inline-flex min-h-8 cursor-pointer items-center hover:underline">
            Columns
          </summary>
          <fieldset className="bg-popover lift-menu absolute z-10 mt-1 flex min-w-48 flex-col gap-1 rounded-md border p-2">
            <legend className="sr-only">Columns to show</legend>
            {every.map((column) => (
              <label key={column.name} className="inline-flex items-center gap-1.5 font-mono">
                <input
                  type="checkbox"
                  checked={shown.includes(column.name)}
                  onChange={(event) => pick(column.name, event.target.checked)}
                />
                {column.name}
              </label>
            ))}
            {view.fields !== null && (
              <button
                type="button"
                className="text-primary mt-1 self-start"
                onClick={() => onView({ long: false, fields: null })}
              >
                Default columns
              </button>
            )}
          </fieldset>
        </details>
      </div>
      <div className="overflow-x-auto">
        <table aria-label={label} className="w-auto border-collapse text-left">
          <thead>
            <tr>
              <th className="w-6 border-b p-1">
                <span className="sr-only">Record</span>
              </th>
              {columns.map((column) => (
                <th
                  key={column.name}
                  scope="col"
                  className={cn(
                    "border-b px-2.5 py-1.5 align-bottom",
                    numeric.has(column.name) && "text-right",
                  )}
                >
                  <span className="block font-mono text-xs font-semibold">{column.name}</span>
                  {column.description !== null && (
                    <span className="text-muted-foreground hidden max-w-44 text-xs font-normal md:block">
                      {column.description}
                    </span>
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {numbered(entries).map((item) =>
              item.kind === "failure" ? (
                <tr key={item.position}>
                  <td colSpan={columns.length + 1} className="border-b px-1 py-1.5">
                    {drawFailure(item.failure)}
                  </td>
                </tr>
              ) : (
                <Fragment key={item.position}>
                  <tr className={open === item.index ? "bg-accent" : undefined}>
                    <td className="border-b p-1 align-top">
                      <button
                        type="button"
                        aria-expanded={open === item.index}
                        aria-label={`Full record of row ${item.index + 1}`}
                        onClick={() => setOpen(open === item.index ? null : item.index)}
                        className="text-faint-foreground hover:bg-muted hover:text-foreground transition-tint rounded p-0.5"
                      >
                        {open === item.index ? (
                          <ChevronDown aria-hidden="true" className="size-3.5" />
                        ) : (
                          <ChevronRight aria-hidden="true" className="size-3.5" />
                        )}
                      </button>
                    </td>
                    {columns.map((column) => (
                      <td
                        key={column.name}
                        className={
                          typeof item.row[column.name] === "number"
                            ? "border-b px-2.5 py-1.5 text-right align-top font-mono tabular-nums"
                            : "border-b px-2.5 py-1.5 align-top"
                        }
                      >
                        <Cell
                          value={item.row[column.name]}
                          link={isLinkColumn(column.name) ? link : null}
                          onOpenLink={onOpenLink}
                        />
                      </td>
                    ))}
                  </tr>
                  {open === item.index && (
                    <tr>
                      <td />
                      <td colSpan={columns.length} className="border-b px-2.5 py-2">
                        <RecordView record={item.row} label={`Row ${item.index + 1}`} />
                      </td>
                    </tr>
                  )}
                </Fragment>
              ),
            )}
          </tbody>
        </table>
      </div>
      {(footer !== null || sizes.length > 0) && (
        <div className="text-muted-foreground flex flex-wrap items-center gap-3 text-xs">
          {footer !== null && <span>{footer}</span>}
          {sizes.length > 0 && (
            <span className="inline-flex items-center gap-1">
              Rows:
              {sizes.map((size) => (
                <button
                  key={size}
                  type="button"
                  className="text-foreground hover:bg-muted active:bg-accent transition-tint inline-flex min-h-8 items-center rounded border px-2"
                  onClick={() => onPageSize(size)}
                >
                  {sizeLabels[size]}
                </button>
              ))}
            </span>
          )}
        </div>
      )}
    </div>
  );
}

type CellProps = { value: unknown; link: Link | null; onOpenLink: (value: string) => void };

/**
 * One cell. An `id` cell that an inferred link starts from has a dotted
 * underline and fills the linked command's composer when pressed. It runs
 * nothing.
 */
function Cell({ value, link, onOpenLink }: CellProps) {
  if (link === null || (typeof value !== "string" && typeof value !== "number")) {
    return <CellValue value={value} />;
  }
  const text = String(value);
  const words = cellLinkWords(link, text);
  return (
    <button
      type="button"
      title={words}
      aria-label={words}
      onClick={() => onOpenLink(text)}
      className="-my-1.5 inline-flex min-h-8 cursor-pointer items-center underline decoration-dotted underline-offset-4"
    >
      {text}
    </button>
  );
}

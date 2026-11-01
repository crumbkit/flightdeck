import { ChevronDown, ChevronRight } from "lucide-react";
import { Fragment, useState } from "react";
import type { Command } from "@/api/model";
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

type Props = {
  command: Command;
  rows: Row[];
  listing: { total: number | null; truncated: boolean } | null;
  view: View;
  onView: (view: View) => void;
  onPageSize: (size: PageSize) => void;
  label: string;
};

const sizeLabels: Record<PageSize, string> = {
  default: "Default",
  "25": "25",
  "100": "100",
  all: "All",
};

/**
 * A result's rows as a table. Each column shows its description under its
 * name. The toolbar adds the long columns or picks columns, which the card
 * adds to its copied line, and each row expands to its full record. A
 * listing ends with crumbkit's footer and its page sizes, each of which
 * runs the command again in a new card.
 */
export function ResultTable({ command, rows, listing, view, onView, onPageSize, label }: Props) {
  const [open, setOpen] = useState<number | null>(null);
  const columns = columnsOf(command, rows, view);
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
          <label className="inline-flex items-center gap-1.5">
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
          <summary className="text-primary cursor-pointer">Columns</summary>
          <fieldset className="bg-card absolute z-10 mt-1 flex min-w-48 flex-col gap-1 rounded-md border p-2 shadow-md">
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
        <table aria-label={label} className="w-full border-collapse text-left">
          <thead>
            <tr>
              <th className="w-6 border-b p-1">
                <span className="sr-only">Record</span>
              </th>
              {columns.map((column) => (
                <th key={column.name} scope="col" className="border-b px-2.5 py-1.5 align-bottom">
                  <span className="text-faint-foreground block text-xs font-semibold tracking-wide uppercase">
                    {column.name}
                  </span>
                  {column.description !== null && (
                    <span className="text-muted-foreground block text-xs font-normal">
                      {column.description}
                    </span>
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((row, index) => (
              // Rows have no key of their own, and a result's rows never change order.
              // biome-ignore lint/suspicious/noArrayIndexKey: a result's rows are fixed
              <Fragment key={index}>
                <tr className={open === index ? "bg-accent" : undefined}>
                  <td className="border-b p-1 align-top">
                    <button
                      type="button"
                      aria-expanded={open === index}
                      aria-label={`Full record of row ${index + 1}`}
                      onClick={() => setOpen(open === index ? null : index)}
                      className="text-faint-foreground rounded p-0.5"
                    >
                      {open === index ? (
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
                        typeof row[column.name] === "number"
                          ? "border-b px-2.5 py-1.5 text-right align-top tabular-nums"
                          : "border-b px-2.5 py-1.5 align-top"
                      }
                    >
                      <CellValue value={row[column.name]} />
                    </td>
                  ))}
                </tr>
                {open === index && (
                  <tr>
                    <td />
                    <td colSpan={columns.length} className="border-b px-2.5 py-2">
                      <RecordView record={row} label={`Row ${index + 1}`} />
                    </td>
                  </tr>
                )}
              </Fragment>
            ))}
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
                  className="text-foreground rounded border px-1.5"
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

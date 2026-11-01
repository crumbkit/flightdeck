import type { Column, Command } from "@/api/model";
import type { RunRequest, Value } from "@/api/runs";

/** How a card shows a table: with the long columns, or with picked fields. */
export type View = { long: boolean; fields: string[] | null };

/** A row of a result, as the tool printed it. */
export type Row = Record<string, unknown>;

/**
 * Returns the columns a table shows. Picked fields win, in the order
 * picked. Otherwise the table shows the command's compact columns, and its
 * long columns too when the card asks for them. A command that declares no
 * columns gets the fields of its first row.
 */
export function columnsOf(command: Command, rows: Row[], view: View): Column[] {
  const declared = allColumnsOf(command, rows);
  if (view.fields !== null) {
    return view.fields.map(
      (name) =>
        declared.find((column) => column.name === name) ?? { name, long: false, description: null },
    );
  }
  return declared.filter((column) => view.long || !column.long);
}

/** Returns every column a column picker offers, compact ones first. */
export function allColumnsOf(command: Command, rows: Row[]): Column[] {
  if (command.result.columns.length > 0) {
    return command.result.columns;
  }
  const [first] = rows;
  return Object.keys(first ?? {}).map((name) => ({ name, long: false, description: null }));
}

/**
 * Returns the footer of a listing as crumbkit words it, such as "Showing 4
 * of 6 books", or null when the listing shows every row.
 *
 * crumbkit names the rows with a phrase that its JSON leaves out, so the
 * page names them with the command's resource, the word before its verb,
 * such as "books" for `books list`. A phrase in the model replaces this
 * when crumbkit prints one.
 */
export function footerOf(
  command: Command,
  shown: number,
  listing: { total: number | null; truncated: boolean },
): string | null {
  if (!listing.truncated) {
    return null;
  }
  const noun = command.path.at(-2) ?? "rows";
  return listing.total === null
    ? `Showing ${shown} ${noun}, and more match`
    : `Showing ${shown} of ${listing.total} ${noun}`;
}

/** Returns a run's line with the flags that choose the card's columns. */
export function copiedLine(line: string, view: View): string {
  if (view.fields !== null) {
    return `${line} --fields ${view.fields.join(",")}`;
  }
  return view.long ? `${line} --long` : line;
}

/** A page size of a listing: its default, a number of rows, or every row. */
export type PageSize = "default" | "25" | "100" | "all";

/** Returns the page sizes a command offers: none unless it takes `--limit` and `--all`. */
export function pageSizesOf(command: Command): PageSize[] {
  const takes = (flag: string) => command.arguments.some((each) => each.crumbkit_flag === flag);
  return takes("--limit") && takes("--all") ? ["default", "25", "100", "all"] : [];
}

/** Returns `request` with its paging flags replaced by `size`. */
export function withPageSize(request: RunRequest, size: PageSize): RunRequest {
  const { "--limit": _limit, "--all": _all, ...values } = request.values;
  const paging: Record<string, Value> = {};
  if (size === "all") {
    paging["--all"] = true;
  } else if (size !== "default") {
    paging["--limit"] = size;
  }
  return { ...request, values: { ...values, ...paging } };
}

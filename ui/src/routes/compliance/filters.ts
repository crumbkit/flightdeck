// The Compliance page's filters beside the Show chips that the headless
// view holds: a count tile, which keeps the matrix's rows with a cell in its
// state or, for No report, the repositories without a report, and the Group
// chips, which keep the repositories of one group. Both live in the
// page's address.

import type { ComplianceDocument } from "@crumbkit/flightdeck-headless/document";
import type { MatrixRow } from "@crumbkit/flightdeck-headless/matrix";
import { defineAddress, oneOf, text } from "@crumbkit/ui/address";

/** The tiles that narrow the matrix. */
export const TILES = ["failing", "stale", "current", "no_report"] as const;

/** The tile and group filters' address: `?tile=failing&group=Platform`. */
export const filtersAddress = defineAddress({
  tile: oneOf(["all", ...TILES], "all"),
  group: text(""),
});

/** The values the tile and group filters hold. */
export type Filters = ReturnType<typeof filtersAddress.parse>;

/** Returns the groups the repositories name, in the order they first appear. */
export function groupsOf(document: ComplianceDocument): string[] {
  const groups: string[] = [];
  for (const column of document.repositories) {
    if (column.group !== null && !groups.includes(column.group)) groups.push(column.group);
  }
  return groups;
}

/** Returns the indexes of the repositories the filters keep, in the document's order. */
export function shownColumns(document: ComplianceDocument, filters: Filters): number[] {
  return document.repositories.flatMap((column, index) => {
    if (filters.group !== "" && column.group !== filters.group) return [];
    if (
      filters.tile === "no_report" &&
      column.report !== "no_report" &&
      column.report !== "invalid_report"
    ) {
      return [];
    }
    return [index];
  });
}

/** Returns the rows the tile keeps: those with a cell in its state among `columns`, or every row for All and No report. */
export function shownRows(
  rows: readonly MatrixRow[],
  filters: Filters,
  columns: readonly number[],
): MatrixRow[] {
  const tile = filters.tile;
  if (tile === "all" || tile === "no_report") return [...rows];
  return rows.filter((row) => columns.some((index) => row.cells[index]?.state === tile));
}

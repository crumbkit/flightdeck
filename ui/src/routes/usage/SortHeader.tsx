import { cn } from "cn";

/** The direction a column sorts in. */
type Direction = "ascending" | "descending";

/** A column header that sorts the table, marked with the direction it sorts in. */
export function SortHeader({
  label,
  sort,
  onSort,
  rowSpan = 1,
  className = "",
}: {
  label: string;
  sort: Direction | null;
  onSort: () => void;
  rowSpan?: number;
  className?: string;
}) {
  return (
    <th
      scope="col"
      rowSpan={rowSpan}
      aria-sort={sort ?? undefined}
      className={cn(
        "border-b px-2.5 py-1.75 text-right align-bottom font-medium whitespace-nowrap",
        className,
      )}
    >
      <button
        type="button"
        onClick={onSort}
        className={cn(
          "hover:text-foreground rounded-xs whitespace-nowrap",
          sort === null ? "text-faint-foreground" : "text-foreground",
        )}
      >
        {label}
        {sort !== null && <span aria-hidden="true">{sort === "descending" ? " ↓" : " ↑"}</span>}
      </button>
    </th>
  );
}

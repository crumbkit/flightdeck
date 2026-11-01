import { durationWords } from "@/lib/duration";

/**
 * One value of a row as the page shows it. A duration shows its words beside
 * the ISO value the tool printed, such as "1 h 40 min" beside PT1H40M.
 */
export function CellValue({ value }: { value: unknown }) {
  if (typeof value === "string") {
    const words = durationWords(value);
    return words === null ? (
      value
    ) : (
      <span className="whitespace-nowrap">
        {words} <code className="text-faint-foreground font-mono text-xs">{value}</code>
      </span>
    );
  }
  if (value === null || value === undefined) {
    return <span className="text-faint-foreground">none</span>;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  return <code className="font-mono text-xs">{JSON.stringify(value)}</code>;
}

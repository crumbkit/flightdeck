import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { type ReactNode, useState } from "react";
import { describeFailure } from "@/api/failure";
import type { Day, Tool } from "@/api/model";
import { weightQuery } from "@/api/weight";
import {
  type BothRow,
  bothRows,
  bothSummary,
  type Corner,
  cornerCounts,
  cornerHints,
  cornerLabels,
  corners,
  readWords,
} from "@/lib/both";
import { grouped } from "@/lib/cost";
import { budgets } from "@/lib/weight";

type Props = {
  tool: Tool;
  /** Opens the form of the command named, such as "books show". */
  onOpen: (command: string) => void;
};

/** A chip's choice: every command, or one corner. */
type Choice = Corner | "all";

/**
 * The tab bar's summary of the both tab: the window and the thresholds of
 * heavy and often. A phone shows only the window.
 */
export function BothSummary({ tool, today }: { tool: Tool; today: Day }) {
  const [period, heavy, often] = bothSummary(tool, today);
  return (
    <>
      {period}
      <span className="hidden md:inline">
        {" · "}
        {heavy}
        {" · "}
        {often}
      </span>
    </>
  );
}

/** Returns the words of a row's reading: the tokens, "at least" for a partial path, or "not counted". */
function readOf(row: BothRow): string {
  if (row.read === null) {
    return "not counted";
  }
  return row.partial && row.read > 0 ? `at least ${readWords(row.read)}` : readWords(row.read);
}

/** A bar of `value` on a scale of `scale`, in `colour`, beside its value. */
function Bar({
  value,
  scale,
  colour,
  children,
  bad = false,
}: {
  value: number;
  scale: number;
  colour: string;
  children: ReactNode;
  bad?: boolean;
}) {
  return (
    <span className="grid-cols-twobar grid items-center gap-2">
      <span aria-hidden="true" className="bg-muted relative h-2 rounded-xs">
        <span
          className={cn("absolute inset-y-0 left-0 rounded-xs", colour)}
          style={{ width: `${(value / scale) * 100}%` }}
        />
      </span>
      <span
        className={cn(
          "text-right font-mono whitespace-nowrap tabular-nums",
          bad && "text-destructive",
        )}
      >
        {children}
      </span>
    </span>
  );
}

/**
 * The both tab: every command's path weight beside its runs, sorted by
 * the bytes agents read, weight times runs, shown as tokens. The chips
 * choose a corner of heavy or light against run often or never. It reads
 * the weight tab's measurement, so the help pages run once. Below the md
 * width the table becomes one list row per command.
 */
export default function Both({ tool, onOpen }: Props) {
  const weight = useQuery(weightQuery());
  const [choice, setChoice] = useState<Choice>("all");
  if (weight.isPending) {
    return (
      <p role="status" className="text-muted-foreground m-0 px-4 py-3 text-sm">
        Measuring {tool.name}'s help pages…
      </p>
    );
  }
  if (weight.isError) {
    return (
      <p role="alert" className="text-destructive m-0 px-4 py-3 text-sm">
        {describeFailure(weight.error)}
      </p>
    );
  }
  if (weight.data.problem !== null) {
    return (
      <p role="alert" className="text-destructive m-0 px-4 py-3 text-sm">
        flightdeck could not measure help pages: {weight.data.problem}
      </p>
    );
  }
  const all = bothRows(tool, weight.data);
  const counts = cornerCounts(all);
  const rows = choice === "all" ? all : all.filter((row) => row.corner === choice);
  const bytesScale = Math.max(1, ...all.map((row) => row.bytes));
  const runsScale = Math.max(1, ...all.map((row) => row.runs ?? 0));
  const command = (row: BothRow) => (
    <button
      type="button"
      onClick={() => onOpen(row.command)}
      className="text-primary min-h-6 rounded-xs text-left font-mono break-all hover:underline"
    >
      {row.command}
    </button>
  );
  const weightBar = (row: BothRow) => (
    <Bar value={row.bytes} scale={bytesScale} colour="bg-primary" bad={row.bytes > budgets.path}>
      {grouped(row.bytes)} B
    </Bar>
  );
  const runsBar = (row: BothRow) => (
    <Bar value={row.runs ?? 0} scale={runsScale} colour="bg-k-help">
      {row.runs === null ? "–" : grouped(row.runs)}
    </Bar>
  );
  const chip = (each: Choice, label: string, count: number) => (
    <button
      key={each}
      type="button"
      aria-pressed={choice === each}
      onClick={() => setChoice(each)}
      className={cn(
        "inline-flex min-h-8 items-center gap-1.5 rounded-md border px-3 py-1 text-xs",
        choice === each
          ? "bg-accent border-accent-foreground text-accent-foreground font-medium"
          : "bg-muted",
      )}
    >
      {label}{" "}
      <span
        className={cn(
          "font-mono",
          choice === each ? "text-accent-foreground" : "text-muted-foreground",
        )}
      >
        {count}
      </span>
    </button>
  );
  const head = (label: string, className: string) => (
    <th
      scope="col"
      className={cn(
        "text-faint-foreground border-b px-2.5 py-1.75 align-bottom font-medium whitespace-nowrap",
        className,
      )}
    >
      {label}
    </th>
  );
  return (
    <div className="px-4 pt-3 pb-4">
      <fieldset className="m-0 mb-2.5 flex min-w-0 flex-wrap gap-1.5 border-0 p-0">
        <legend className="sr-only">Corners</legend>
        {chip("all", "All", all.length)}
        {corners.map((corner) => chip(corner, cornerLabels[corner], counts[corner]))}
      </fieldset>
      {choice !== "all" && (
        <p className="text-muted-foreground m-0 mb-2.5 text-xs">
          <b className="text-foreground font-medium">{cornerLabels[choice]}:</b>{" "}
          {cornerHints[choice]}
        </p>
      )}
      <div className="bg-card rounded-md border">
        <table
          aria-label="Path weight and runs of each command"
          className="hidden w-full border-collapse text-xs md:table"
        >
          <thead>
            <tr>
              {head("Command", "text-left")}
              {head("Path weight", "w-1/4 text-left")}
              {head("Runs", "w-1/4 text-left")}
              <th
                scope="col"
                aria-sort="descending"
                className="text-foreground border-b px-2.5 py-1.75 text-right align-bottom font-medium whitespace-nowrap"
              >
                Read in 7 days ↓
              </th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.command} className="hover:bg-muted border-b last:border-b-0">
                <td className="px-2.5 py-1.5 align-middle">{command(row)}</td>
                <td className="px-2.5 py-1.5 align-middle">{weightBar(row)}</td>
                <td className="px-2.5 py-1.5 align-middle">{runsBar(row)}</td>
                <td
                  className={cn(
                    "px-2.5 py-1.5 text-right align-middle whitespace-nowrap",
                    row.read === null && "text-muted-foreground",
                  )}
                >
                  {readOf(row)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <ul
          aria-label="Path weight and runs of each command"
          className="m-0 list-none p-0 md:hidden"
        >
          {rows.map((row) => (
            <li key={row.command} className="border-b px-3 py-2.25 last:border-b-0">
              <div className="flex items-baseline justify-between gap-2 text-xs">
                {command(row)}
                <span
                  className={cn(
                    "font-mono whitespace-nowrap",
                    row.read === null && "text-muted-foreground",
                  )}
                >
                  {readOf(row)}
                </span>
              </div>
              <div className="text-muted-foreground grid-cols-bars text-2xs mt-1.5 grid items-center gap-x-2 gap-y-1">
                <span>weight</span>
                {weightBar(row)}
                <span>runs</span>
                {runsBar(row)}
              </div>
            </li>
          ))}
        </ul>
        {rows.length === 0 && (
          <p className="text-muted-foreground m-0 px-3 py-2.5 text-xs">
            No command is in this corner.
          </p>
        )}
      </div>
    </div>
  );
}

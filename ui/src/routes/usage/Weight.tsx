import { useQuery } from "@tanstack/react-query";
import { cn } from "cn";
import { ChevronDown, ChevronRight } from "lucide-react";
import { Fragment, type ReactNode, useState } from "react";
import type { Tool } from "@/api/model";
import { weightQuery } from "@/api/weight";
import { grouped, tokensWords } from "@/lib/cost";
import {
  budgets,
  isOver,
  overWords,
  type Step,
  sortedPaths,
  tokens,
  type WeightKey,
  type WeightPath,
  weightPaths,
  weightSummary,
} from "@/lib/weight";
import { SortHeader } from "./SortHeader";
import { useMeasured } from "./useMeasured";

type Props = { tool: Tool };

/** The words of each kind of step, under its label. */
const kindWords: Record<Step["kind"], string> = {
  root: "root page",
  group: "group page",
  command: "command page",
  output: "p95 of real output",
};

/** Returns the words under a step's label: its kind, or why an output has no weight. */
function stepKind(step: Step): string {
  return step.kind === "output" && step.note !== null ? step.note : kindWords[step.kind];
}

/** Returns whether a step with a page budget is over it. */
function stepOver(step: Step): boolean {
  return step.bytes !== null && step.budget !== null && step.bytes > step.budget;
}

/** Returns `bytes` as text with its unit, such as "1,180 B". */
function bytesText(bytes: number): string {
  return `${grouped(bytes)} B`;
}

/** The text alternative of a path's bar. */
function pathLabel(path: WeightPath): string {
  const sum = path.partial ? `At least ${bytesText(path.total)}` : bytesText(path.total);
  return `${sum} against a ${bytesText(budgets.path)} default budget`;
}

/**
 * A bar of a path's steps, help pages and output in their own colours, on
 * a scale of `scale` bytes, with a tick at the path budget.
 */
function PathBar({ path, scale }: { path: WeightPath; scale: number }) {
  return (
    <span role="img" aria-label={pathLabel(path)} className="relative flex h-2.5 gap-px">
      {path.steps.map((step, index) =>
        step.bytes === null || step.bytes === 0 ? null : (
          <span
            // biome-ignore lint/suspicious/noArrayIndexKey: a path's steps never reorder
            key={index}
            className={cn(
              "block h-full rounded-xs",
              step.kind === "output" ? "bg-k-ok" : "bg-k-help",
            )}
            style={{ width: `${(step.bytes / scale) * 100}%` }}
          />
        ),
      )}
      <span
        aria-hidden="true"
        className="bg-foreground absolute -top-0.75 -bottom-0.75 w-0.5"
        style={{ left: `${(budgets.path / scale) * 100}%` }}
      />
    </span>
  );
}

/** A bar of one step against its page budget, with the part over the budget in the danger colour. */
function StepBar({ step, scale }: { step: Step; scale: number }) {
  if (step.bytes === null) {
    return null;
  }
  // The budget that the step passes, or null for a step within its budget
  // or without one.
  const passed = stepOver(step) ? step.budget : null;
  const filled = passed ?? step.bytes;
  const label =
    step.budget === null
      ? bytesText(step.bytes)
      : `${bytesText(step.bytes)} of a ${bytesText(step.budget)} default budget`;
  return (
    <span role="img" aria-label={label} className="bg-muted relative block h-2 rounded-xs">
      <span
        className={cn(
          "absolute inset-y-0 left-0 rounded-xs",
          step.kind === "output" ? "bg-k-ok" : "bg-k-help",
        )}
        style={{ width: `${(filled / scale) * 100}%` }}
      />
      {passed !== null && (
        <span
          className="bg-destructive absolute inset-y-0 rounded-xs"
          style={{
            left: `${(passed / scale) * 100}%`,
            width: `${((step.bytes - passed) / scale) * 100}%`,
          }}
        />
      )}
      {step.budget !== null && (
        <span
          aria-hidden="true"
          className="bg-foreground absolute -top-0.75 -bottom-0.75 w-0.5"
          style={{ left: `${(step.budget / scale) * 100}%` }}
        />
      )}
    </span>
  );
}

/** Returns the scale of a path's step bars: its largest step or page budget. */
function stepScale(path: WeightPath): number {
  return Math.max(1, ...path.steps.map((step) => Math.max(step.bytes ?? 0, step.budget ?? 0)));
}

/** A path's sum against the path budget: "over by", "under" or "partial". */
function Against({ path }: { path: WeightPath }) {
  return (
    <span
      className={cn(
        "whitespace-nowrap",
        isOver(path) ? "text-destructive font-medium" : "text-muted-foreground",
      )}
    >
      {overWords(path.total, budgets.path, path.partial)}
    </span>
  );
}

/** A step against its page budget, or why it has no bytes. */
function StepAgainst({ step }: { step: Step }) {
  if (step.bytes === null) {
    return <span className="text-destructive">{step.note}</span>;
  }
  if (step.budget === null) {
    return <span className="text-muted-foreground">counts toward the path only</span>;
  }
  return (
    <span className="whitespace-nowrap">
      <span className={stepOver(step) ? "text-destructive font-medium" : "text-muted-foreground"}>
        {overWords(step.bytes, step.budget)}
      </span>{" "}
      <span className="text-muted-foreground">of {bytesText(step.budget)} default budget</span>
    </span>
  );
}

/** The heaviest step of a path: its label, or the output's words. */
function Heaviest({ step }: { step: Step | null }) {
  if (step === null) {
    return null;
  }
  return step.kind === "output" ? (
    <span className="text-muted-foreground">p95 of real output</span>
  ) : (
    <code>{step.label}</code>
  );
}

/** A step against its page budget in a phone row: the output's tokens, or why it has no bytes. */
function PhoneStepAgainst({ step }: { step: Step }) {
  if (step.bytes === null) {
    return <span className="text-destructive">{step.note}</span>;
  }
  if (step.budget === null) {
    return <span>{tokensWords(step.bytes)}</span>;
  }
  return (
    <span className={cn(stepOver(step) && "text-destructive font-medium")}>
      {overWords(step.bytes, step.budget)}
    </span>
  );
}

/**
 * A step's kind, with a mark on the heaviest step, drawn on `tag` so it
 * stands out from the row behind it.
 */
function KindLine({ step, heaviest, tag }: { step: Step; heaviest: boolean; tag: string }) {
  return (
    <span>
      {stepKind(step)}
      {heaviest && (
        <span className={cn(tag, "text-accent-foreground ml-1.5 rounded-md px-1.5 font-medium")}>
          heaviest
        </span>
      )}
    </span>
  );
}

/** A step's label, with its kind under it and a mark on the heaviest step. */
function StepLabel({ step, heaviest }: { step: Step; heaviest: boolean }) {
  return (
    <span className="flex flex-col">
      <code>{step.label}</code>
      <span className="text-muted-foreground text-2xs">
        <KindLine step={step} heaviest={heaviest} tag="bg-accent" />
      </span>
    </span>
  );
}

/** The button that opens or closes a path's steps. */
function Disclose({
  path,
  open,
  onToggle,
}: {
  path: WeightPath;
  open: boolean;
  onToggle: () => void;
}) {
  const Icon = open ? ChevronDown : ChevronRight;
  return (
    <button
      type="button"
      aria-expanded={open}
      onClick={onToggle}
      className="inline-flex min-h-6 items-center gap-1.5 rounded-xs text-left"
    >
      <Icon aria-hidden="true" className="text-muted-foreground size-3 shrink-0" />
      <span
        className={cn(
          "font-mono whitespace-nowrap",
          open ? "text-accent-foreground font-semibold" : "text-primary",
        )}
      >
        {path.command}
      </span>
    </button>
  );
}

function Cell({ className, children }: { className?: string; children?: ReactNode }) {
  return <td className={cn("px-2.5 py-1.5 align-middle", className)}>{children}</td>;
}

function Count({ bad = false, children }: { bad?: boolean; children: ReactNode }) {
  return (
    <Cell
      className={cn(
        "text-right font-mono whitespace-nowrap tabular-nums",
        bad && "text-destructive",
      )}
    >
      {children}
    </Cell>
  );
}

/**
 * The tab bar's summary of the weight tab, which reads the same query as
 * the tab, so the help pages run once.
 */
export function WeightSummary({ tool }: Props) {
  const weight = useQuery(weightQuery());
  if (weight.data === undefined || weight.data.problem !== null) {
    return null;
  }
  return <>{weightSummary(weightPaths(tool, weight.data), weight.data)}</>;
}

/**
 * The weight tab: for each command, the help pages a reader passes to
 * reach it and the 95th percentile of its real output, against crumbkit's
 * default budgets. It asks the server to measure the help pages the first
 * time it opens, and keeps them until the page reloads. A row opens to
 * show each step. Below the md width the table becomes one list row per
 * command.
 */
export default function Weight({ tool }: Props) {
  const measured = useMeasured(tool);
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const [sort, setSort] = useState<{ key: WeightKey; direction: "ascending" | "descending" }>({
    key: "bytes",
    direction: "descending",
  });
  if (measured.reply === null) {
    return measured.instead;
  }
  const paths = sortedPaths(weightPaths(tool, measured.reply), sort.key, sort.direction);
  const scale = Math.max(budgets.path, ...paths.map((path) => path.total));
  const toggle = (command: string) => {
    const next = new Set(open);
    if (!next.delete(command)) {
      next.add(command);
    }
    setOpen(next);
  };
  const choose = (key: WeightKey) =>
    setSort(
      key === sort.key
        ? { key, direction: sort.direction === "ascending" ? "descending" : "ascending" }
        : { key, direction: key === "command" ? "ascending" : "descending" },
    );
  const header = (key: WeightKey, label: string, className = "") => (
    <SortHeader
      label={label}
      sort={key === sort.key ? sort.direction : null}
      onSort={() => choose(key)}
      className={className}
    />
  );
  const plain = (label: string, className = "text-left") => (
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
      {measured.notice}
      <div className="text-muted-foreground mb-2.5 flex flex-wrap gap-x-3.5 gap-y-1.5 text-xs">
        <span className="inline-flex items-center gap-1.5">
          <span aria-hidden="true" className="bg-k-help size-3 shrink-0 rounded-xs" />
          help page
        </span>
        <span className="inline-flex items-center gap-1.5">
          <span aria-hidden="true" className="bg-k-ok size-3 shrink-0 rounded-xs" />
          p95 of real output
        </span>
        <span className="inline-flex items-center gap-1.5">
          <span aria-hidden="true" className="bg-foreground h-3.5 w-0.5 shrink-0" />
          <span className="md:hidden">default budget</span>
          <span className="hidden md:inline">
            default budget: {bytesText(budgets.path)} a path, {bytesText(budgets.root)} the root
            page, {bytesText(budgets.group)} a group page, {bytesText(budgets.command)} a command
            page
          </span>
        </span>
      </div>
      <div className="bg-card rounded-md border">
        <table
          aria-label="Weight of each command's path"
          className="hidden w-full border-collapse text-xs md:table"
        >
          <thead>
            <tr>
              {header("command", "Command", "w-1/5 text-left")}
              {plain("Path", "w-3/10 text-left")}
              {header("bytes", "Bytes")}
              {plain("Tokens", "text-right")}
              {plain("Against budget")}
              {plain("Heaviest step")}
            </tr>
          </thead>
          <tbody>
            {paths.map((path) => {
              const opened = open.has(path.command);
              const steps = stepScale(path);
              return (
                <Fragment key={path.command}>
                  <tr className={cn("border-b", opened ? "bg-accent" : "hover:bg-muted")}>
                    <Cell>
                      <Disclose path={path} open={opened} onToggle={() => toggle(path.command)} />
                    </Cell>
                    <Cell>
                      <PathBar path={path} scale={scale} />
                    </Cell>
                    <Count bad={isOver(path)}>{grouped(path.total)}</Count>
                    <Count>{grouped(tokens(path.total))}</Count>
                    <Cell>
                      <Against path={path} />
                    </Cell>
                    <Cell className="whitespace-nowrap">
                      <Heaviest step={path.heaviest} />
                    </Cell>
                  </tr>
                  {opened &&
                    path.steps.map((step, index) => (
                      <tr
                        // biome-ignore lint/suspicious/noArrayIndexKey: a path's steps never reorder
                        key={index}
                        className="bg-background border-b"
                      >
                        <Cell className="pl-8">
                          <StepLabel step={step} heaviest={step === path.heaviest} />
                        </Cell>
                        <Cell>
                          <StepBar step={step} scale={steps} />
                        </Cell>
                        <Count bad={stepOver(step)}>
                          {step.bytes === null ? "–" : grouped(step.bytes)}
                        </Count>
                        <Count>{step.bytes === null ? "–" : grouped(tokens(step.bytes))}</Count>
                        <Cell>
                          <StepAgainst step={step} />
                        </Cell>
                        <Cell />
                      </tr>
                    ))}
                  {opened && (
                    <tr className="bg-background border-b">
                      <Cell className="pl-8 font-semibold">Path</Cell>
                      <Cell />
                      <Count bad={isOver(path)}>
                        <b>{grouped(path.total)}</b>
                      </Count>
                      <Count>
                        <b>{grouped(tokens(path.total))}</b>
                      </Count>
                      <Cell>
                        <Against path={path} />{" "}
                        <span className="text-muted-foreground">
                          of {bytesText(budgets.path)} default budget
                        </span>
                      </Cell>
                      <Cell />
                    </tr>
                  )}
                </Fragment>
              );
            })}
          </tbody>
        </table>
        <ul aria-label="Weight of each command's path" className="m-0 list-none p-0 md:hidden">
          {paths.map((path) => {
            const opened = open.has(path.command);
            return (
              <li
                key={path.command}
                className={cn("border-b px-3 py-2.25 last:border-b-0", opened && "bg-accent")}
              >
                <div className="flex items-baseline justify-between gap-2 text-xs">
                  <Disclose path={path} open={opened} onToggle={() => toggle(path.command)} />
                  <span
                    className={cn("font-mono tabular-nums", isOver(path) && "text-destructive")}
                  >
                    {bytesText(path.total)}
                  </span>
                </div>
                <div className="mt-1.5">
                  <PathBar path={path} scale={scale} />
                </div>
                <div className="text-muted-foreground text-2xs mt-1.25 flex justify-between gap-2">
                  <span>{tokensWords(path.total)}</span>
                  <Against path={path} />
                </div>
                {opened && (
                  <ul className="border-input m-0 mt-2 list-none border-l py-0 pr-0 pl-2.5">
                    {path.steps.map((step, index) => (
                      // biome-ignore lint/suspicious/noArrayIndexKey: a path's steps never reorder
                      <li key={index} className="py-1.25">
                        <div className="flex justify-between gap-2 text-xs">
                          <code className="min-w-0 break-all">{step.label}</code>
                          <span className={cn("font-mono", stepOver(step) && "text-destructive")}>
                            {step.bytes === null ? "–" : bytesText(step.bytes)}
                          </span>
                        </div>
                        <div className="text-muted-foreground text-2xs mt-0.75 flex justify-between gap-2">
                          <KindLine step={step} heaviest={step === path.heaviest} tag="bg-card" />
                          <PhoneStepAgainst step={step} />
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
      </div>
      <p className="text-faint-foreground m-0 mt-2.5 text-xs">
        Help pages come from running each command with <code>-h</code> and a cleared environment.
        Tokens are bytes divided by 4.
      </p>
    </div>
  );
}

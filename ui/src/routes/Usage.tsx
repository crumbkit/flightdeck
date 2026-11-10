import { cn } from "cn";
import { lazy, type ReactNode, Suspense, useState } from "react";
import type { Day, Run, Tool } from "@/api/model";
import { ViewButton } from "@/components/ViewButton";
import { bytesWords, dayOf, dayWords, hasFigures, type UsageTab } from "@/lib/cost";
import { elapsedWords } from "@/lib/duration";
import { figureRows, figuresSummary } from "@/lib/figures";
import {
  type FlagRow,
  flagRows,
  gapWords,
  loops,
  neighbours,
  type RunKind,
  type Session,
  sessionsOf,
  spanWords,
  stepsMatrix,
  timeWords,
} from "@/lib/usage";
import { counted } from "@/lib/words";
import { Figures } from "./usage/Figures";

type Props = {
  tool: Tool;
  /** Each run in the tool's usage log, or null when the tool lists none. */
  history: Run[] | null;
  tab: UsageTab;
  /** The command the view is about, such as "books show". */
  command: string;
  today: Day;
  onTab: (tab: UsageTab) => void;
  onCommand: (command: string) => void;
  /** Opens the form of the command named, such as "books show". */
  onOpen: (command: string) => void;
};

/**
 * The tabs about every command. Both shows only when the tool reports
 * figures, because it multiplies by runs.
 */
const everyCommand: [UsageTab, string][] = [
  ["figures", "Figures"],
  ["weight", "Weight"],
  ["both", "Both"],
];

// The weight and both tabs each load in their own file the first time they
// open, so the usage view's file stays within its size budget. Each
// summary comes from its tab's file. Both reads the weight tab's query, so
// the help pages run once.
const Weight = lazy(() => import("./usage/Weight"));
const WeightSummary = lazy(() =>
  import("./usage/Weight").then((module) => ({ default: module.WeightSummary })),
);
const Both = lazy(() => import("./usage/Both"));
const BothSummary = lazy(() =>
  import("./usage/Both").then((module) => ({ default: module.BothSummary })),
);

/** The tabs about one command, which show when the tool lists its runs. */
const oneCommand: [UsageTab, string][] = [
  ["steps", "Steps"],
  ["sessions", "Sessions"],
  ["flags", "Flags"],
];

/** The most commands the steps matrix shows. */
const matrixSize = 11;

/** The most session lanes the sessions view shows. */
const laneLimit = 8;

/**
 * The usage view: what agents ran. Figures is a table of every command's
 * figures, from the tool's `self figures list`. Weight measures each
 * command's path through the help, and Both sets that weight beside the
 * command's runs. When the tool lists its
 * runs in `self runs list`, a second group of tabs shows the steps between
 * commands, each session's runs and the flags each command's runs passed.
 * It loads apart from the page, the first time it opens.
 */
export default function Usage({
  tool,
  history,
  tab,
  command,
  today,
  onTab,
  onCommand,
  onOpen,
}: Props) {
  const runs = history ?? [];
  const sessions = sessionsOf(runs);
  // A reduce, because a busy log can hold more runs than a call takes arguments.
  const first = runs.reduce((least, run) => Math.min(least, run.at), Number.POSITIVE_INFINITY);
  const days =
    runs.length === 0 ? "" : `${dayWords(dayOf(new Date(first * 1000)))} to ${dayWords(today)}`;
  const commands = [...new Set(runs.filter((run) => !run.help).map((run) => run.command))];
  const rows = figureRows(tool.commands);
  const figured = hasFigures(tool);
  const summary: Record<UsageTab, ReactNode> = {
    figures: figured ? figuresSummary(rows, tool.figures_since, today) : "",
    weight: (
      <Suspense fallback={null}>
        <WeightSummary tool={tool} />
      </Suspense>
    ),
    both: (
      <Suspense fallback={null}>
        <BothSummary tool={tool} today={today} />
      </Suspense>
    ),
    steps: `${counted(runs.length, "run")} in ${counted(sessions.length, "session")}, ${days}`,
    sessions: `${counted(sessions.length, "session")}, ${days}.`,
    flags: `Which flags each command's runs passed, ${days}. The log keeps flag names, never their values.`,
  };
  const button = ([each, label]: [UsageTab, string]) => (
    <ViewButton key={each} on={each === tab} onClick={() => onTab(each)}>
      {label}
    </ViewButton>
  );
  const perCommand = tab !== "figures" && tab !== "weight" && tab !== "both";
  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-auto">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b px-4 py-2">
        <nav aria-label="Usage views" className="flex flex-wrap items-center gap-1">
          {everyCommand.filter(([each]) => each !== "both" || figured).map(button)}
          {history !== null && (
            <>
              <span aria-hidden="true" className="bg-border mx-1 hidden h-5 w-px md:block" />
              {oneCommand.map(button)}
            </>
          )}
        </nav>
        <span className="text-muted-foreground text-xs">{summary[tab]}</span>
        {(tab === "steps" || tab === "sessions") && commands.length > 0 && (
          <label className="text-muted-foreground ml-auto flex items-center gap-2 text-xs">
            Command
            <select
              value={command}
              onChange={(event) => onCommand(event.target.value)}
              className="bg-code text-muted-foreground h-6 rounded-sm px-1.5 font-mono text-xs"
            >
              {commands.map((each) => (
                <option key={each} value={each}>
                  {each}
                </option>
              ))}
            </select>
          </label>
        )}
      </div>
      {tab === "weight" && (
        <Suspense
          fallback={<p className="text-muted-foreground m-0 px-4 py-3 text-sm">Loading…</p>}
        >
          <Weight tool={tool} />
        </Suspense>
      )}
      {tab === "both" && figured && (
        <Suspense
          fallback={<p className="text-muted-foreground m-0 px-4 py-3 text-sm">Loading…</p>}
        >
          <Both tool={tool} onOpen={onOpen} />
        </Suspense>
      )}
      {tab === "figures" &&
        (figured ? (
          <Figures name={tool.name} rows={rows} onOpen={onOpen} />
        ) : (
          <p className="text-muted-foreground m-0 px-4 py-3 text-sm">
            {tool.name} keeps no usage figures.
          </p>
        ))}
      {perCommand && runs.length === 0 && (
        <p className="text-muted-foreground m-0 px-4 py-3 text-sm">
          {tool.name}'s usage log holds no runs in the last 7 days.
        </p>
      )}
      {runs.length > 0 && tab === "steps" && <Steps history={runs} command={command} />}
      {runs.length > 0 && tab === "sessions" && <Sessions sessions={sessions} command={command} />}
      {runs.length > 0 && tab === "flags" && (
        <Flags tool={tool} history={runs} command={command} commands={commands} />
      )}
    </div>
  );
}

/**
 * Returns the background and text colour of a matrix cell that counts
 * `count` steps. The shade grows 3.2 points a step from 12% and is full
 * from 28 steps, as the agreed mockup draws it, so one very common step
 * does not wash out the rest.
 */
function cellStyle(count: number, focus: boolean) {
  const percent = count === 0 ? 0 : Math.min(100, Math.round(12 + count * 3.2));
  return {
    background: `color-mix(in oklch, var(--primary) ${percent}%, var(--muted))`,
    color: percent > 55 ? "var(--primary-foreground)" : "var(--foreground)",
    boxShadow: focus ? "inset 0 0 0 1px var(--primary)" : undefined,
  };
}

function Steps({ history, command }: { history: Run[]; command: string }) {
  const matrix = stepsMatrix(history, matrixSize);
  const around = neighbours(history, command);
  const left = matrix.total - matrix.shown;
  return (
    <div className="flex flex-wrap">
      <section aria-labelledby="steps-matrix" className="flex-matrix min-w-0 px-4 py-3">
        <h2 id="steps-matrix" className="m-0 mb-2 text-sm font-semibold">
          After each command, what the agent ran next
        </h2>
        <p className="text-muted-foreground m-0 mb-2.5 text-xs">
          Rows are the command that ran first, columns the next run in the same session.
        </p>
        <div className="overflow-x-auto">
          <table className="border-separate border-spacing-0.5 font-mono text-xs">
            <thead>
              <tr>
                <th scope="col" className="text-faint-foreground px-1.5 py-1 font-medium">
                  then →
                </th>
                {matrix.keys.map((key) => (
                  <th
                    key={key}
                    scope="col"
                    className={cn(
                      "px-1.5 py-1 font-medium whitespace-nowrap",
                      key === command
                        ? "text-accent-foreground font-semibold"
                        : "text-muted-foreground",
                    )}
                  >
                    {key}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {matrix.keys.map((from, row) => (
                <tr key={from}>
                  <th
                    scope="row"
                    className={cn(
                      "px-1.5 py-1 text-right font-medium whitespace-nowrap",
                      from === command
                        ? "text-accent-foreground font-semibold"
                        : "text-muted-foreground",
                    )}
                  >
                    {from}
                  </th>
                  {matrix.keys.map((to, column) => {
                    const count = matrix.cells[row]?.[column] ?? 0;
                    return (
                      <td
                        key={to}
                        className="h-8 w-13 rounded-xs text-center tabular-nums"
                        style={cellStyle(count, from === command || to === command)}
                      >
                        {count === 0 ? "" : count}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {left > 0 && (
          <p className="text-faint-foreground m-0 mt-2.5 text-xs">
            {matrix.shown} of {counted(matrix.total, "step")}. The other {left} involve commands
            with few runs.
          </p>
        )}
      </section>
      <aside
        aria-label={`Steps around ${command}`}
        className="flex-aside flex max-w-95 flex-col gap-4.5 border-l px-4 py-3"
      >
        <Counts
          title={
            <>
              After <code>{command}</code>
            </>
          }
          rows={around.after}
          bars
        />
        <Counts
          title={
            <>
              Before <code>{command}</code>
            </>
          }
          rows={around.before}
          bars
        />
        <Counts
          title="Loops"
          rows={loops(history).map(([key, count]) => [
            <span key={key}>
              <code>{key}</code> then <code>{key}</code>
            </span>,
            count,
          ])}
        />
      </aside>
    </div>
  );
}

/** A titled list of names and counts, with a bar for each count when `bars` is set. */
function Counts({
  title,
  rows,
  bars = false,
}: {
  title: ReactNode;
  rows: [ReactNode, number][];
  bars?: boolean;
}) {
  const most = Math.max(1, ...rows.map(([, count]) => count));
  return (
    <section>
      <h2 className="m-0 mb-2 text-sm font-semibold">{title}</h2>
      {rows.length === 0 ? (
        <p className="text-muted-foreground m-0 text-xs">None.</p>
      ) : (
        <dl className="grid-cols-counts m-0 grid gap-x-3 gap-y-0.5 text-xs tabular-nums">
          {rows.map(([name, count], index) => (
            // A row's name is unique within its list, but may be an element.
            // biome-ignore lint/suspicious/noArrayIndexKey: the rows never reorder while shown
            <Pair key={index} count={count}>
              {bars ? (
                <span className="grid-cols-bar grid items-center gap-2">
                  <code className="truncate">{name}</code>
                  <span className="bg-muted relative h-2 rounded-xs">
                    <span
                      className="bg-primary absolute inset-y-0 left-0 rounded-xs"
                      style={{ width: `${Math.round((count / most) * 100)}%` }}
                    />
                  </span>
                </span>
              ) : (
                name
              )}
            </Pair>
          ))}
        </dl>
      )}
    </section>
  );
}

function Pair({ count, children }: { count: number; children: ReactNode }) {
  return (
    <>
      <dt className="min-w-0">{children}</dt>
      <dd className="m-0 text-right font-mono">{count.toLocaleString("en")}</dd>
    </>
  );
}

/** The colour of each kind of run, as the legend names it. */
const kinds: [RunKind, string, string][] = [
  ["ok", "bg-k-ok", "succeeded"],
  ["help", "bg-k-help", "help page"],
  ["usage", "bg-k-usage", "usage error"],
  ["fail", "bg-k-fail", "failed"],
  ["repeat", "bg-k-repeat", "same call again within 60 s"],
  ["retry", "bg-k-retry", "ran again after a failure"],
];

function colourOf(kind: RunKind): string {
  return kinds.find(([each]) => each === kind)?.[1] ?? "bg-k-ok";
}

/** Returns a session's runs, failures and repeats in words. */
function laneWords(session: Session): string {
  const of = (...wanted: RunKind[]) => session.kinds.filter((kind) => wanted.includes(kind)).length;
  return `${counted(session.runs.length, "run")}, ${of("fail")} failed, ${of("repeat", "retry")} repeated`;
}

function Sessions({ sessions, command }: { sessions: Session[]; command: string }) {
  const ran = sessions.filter((session) => session.runs.some((run) => run.command === command));
  const shown = ran.slice(0, laneLimit);
  const [open, setOpen] = useState<number | null>(null);
  const expanded = shown.find((session) => session.number === open) ?? shown[0];
  return (
    <>
      <p className="text-muted-foreground m-0 px-4 pt-2 text-xs">
        Showing {shown.length} of the {ran.length} that ran <code>{command}</code>, the most recent
        first.
      </p>
      <div className="text-muted-foreground flex flex-wrap gap-x-3.5 gap-y-1.5 px-4 py-2.5 text-xs">
        {kinds.map(([kind, colour, label]) => (
          <span key={kind} className="inline-flex items-center gap-1.5">
            <span aria-hidden="true" className={cn("size-3 shrink-0 rounded-xs", colour)} />
            {label}
          </span>
        ))}
      </div>
      <div className="bg-card mx-4 rounded-md border">
        {shown.map((session) => (
          <button
            key={session.number}
            type="button"
            aria-pressed={session === expanded}
            onClick={() => setOpen(session.number)}
            className={cn(
              "grid-cols-lane grid w-full items-center gap-x-3 gap-y-2 border-b px-3 py-2 text-left last:border-b-0",
              session === expanded ? "bg-accent" : "hover:bg-muted",
            )}
          >
            <span className="flex flex-col text-xs">
              <span>
                <code>Session {session.number}</code>{" "}
                <span className="text-faint-foreground">{session.caller}</span>
              </span>
              <span className="text-muted-foreground">
                {spanWords(session.first, session.last)}
              </span>
              <span className="text-muted-foreground">{laneWords(session)}</span>
            </span>
            <span
              role="img"
              aria-label={`Session ${session.number}: ${laneWords(session)}`}
              className="flex flex-wrap gap-0.75"
            >
              {session.kinds.map((kind, index) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: a session's runs never reorder
                <span key={index} className={cn("h-5 w-3 rounded-xs", colourOf(kind))} />
              ))}
            </span>
          </button>
        ))}
      </div>
      {expanded !== undefined && <SessionRuns session={expanded} />}
    </>
  );
}

/** Returns a run's detail in the run list: its code, "help", or its time and output. */
function detailOf(run: Run): string {
  if (run.code !== null) {
    return run.code;
  }
  if (run.help) {
    return "help";
  }
  return run.out_bytes === null
    ? elapsedWords(run.ms)
    : `${elapsedWords(run.ms)} · ${bytesWords(run.out_bytes)}`;
}

function SessionRuns({ session }: { session: Session }) {
  const of = (kind: RunKind) => session.kinds.filter((each) => each === kind).length;
  return (
    <section aria-labelledby="session-runs" className="flex flex-col gap-2.5 px-4 py-3.5">
      <h2
        id="session-runs"
        className="m-0 flex flex-wrap items-baseline gap-2 text-sm font-semibold"
      >
        <span>
          Session <code>{session.number}</code>
        </span>
        <span className="text-muted-foreground text-xs font-normal">
          {counted(session.runs.length, "run")}: {of("fail")} failed,{" "}
          {counted(of("usage"), "usage error")}, {counted(of("help"), "help page")}
        </span>
      </h2>
      <ol className="bg-card m-0 flex list-none flex-wrap gap-1 rounded-md border p-3">
        {session.runs.map((run, index) => {
          const before = session.runs[index - 1];
          const gap = before === undefined ? null : gapWords(before.at, run.at);
          const name = run.help ? `help ${run.command}` : run.command;
          const kind = session.kinds[index] ?? "ok";
          return (
            <li key={run.step} className="contents">
              {gap !== null && (
                <span className="text-faint-foreground text-2xs flex items-center self-stretch border-l border-dashed px-1.5">
                  {gap}
                </span>
              )}
              <span
                title={`${name} at ${timeWords(run.at)}, ${elapsedWords(run.ms)}`}
                className="bg-card text-2xs inline-flex min-w-19 flex-col gap-0.5 rounded-md border px-1.5 pt-1 pb-1.25 font-mono leading-tight"
              >
                <span className={cn("h-1 rounded-xs", colourOf(kind))} />
                <b className="text-xs font-semibold">{name}</b>
                <span className="text-faint-foreground">{timeWords(run.at)}</span>
                <span className="text-muted-foreground">{detailOf(run)}</span>
              </span>
            </li>
          );
        })}
      </ol>
    </section>
  );
}

function Flags({
  tool,
  history,
  command,
  commands,
}: {
  tool: Tool;
  history: Run[];
  command: string;
  commands: string[];
}) {
  const sections = commands
    .flatMap((name) => {
      const declared = tool.commands.find((each) => each.path.join(" ") === name);
      const table = flagRows(declared?.arguments ?? [], history, name);
      return table.flags.length === 0 ? [] : [{ name, ...table }];
    })
    .sort((a, b) => Number(b.name === command) - Number(a.name === command) || b.runs - a.runs);
  if (sections.length === 0) {
    return (
      <p className="text-muted-foreground m-0 px-4 py-3 text-sm">No command declares a flag.</p>
    );
  }
  return sections.map((section) => (
    <section
      key={section.name}
      aria-label={section.name}
      className="flex flex-col gap-2.5 border-b px-4 py-3.5"
    >
      <h2 className="m-0 flex flex-wrap items-baseline gap-2 text-sm font-semibold">
        <code>
          {tool.name} {section.name}
        </code>
        <span className="text-muted-foreground text-xs font-normal">
          {counted(section.runs, "run")}
        </span>
      </h2>
      <div className="grid-cols-flags grid items-center gap-x-3.5 gap-y-1.5 text-xs">
        <span className="text-faint-foreground font-medium">Flag</span>
        <span className="text-faint-foreground font-medium">Share of runs</span>
        <span className="text-faint-foreground text-right font-medium">Runs</span>
        <span className="text-faint-foreground font-medium">Note</span>
        {section.flags.map((flag) => (
          <FlagLine key={flag.name} flag={flag} runs={section.runs} />
        ))}
      </div>
    </section>
  ));
}

function FlagLine({ flag, runs }: { flag: FlagRow; runs: number }) {
  const share =
    runs === 0 ? 0 : Math.max(flag.runs === 0 ? 0 : 1, Math.round((flag.runs / runs) * 100));
  return (
    <>
      <code>{flag.name}</code>
      <span className="bg-muted relative h-2 rounded-xs">
        <span
          className={cn(
            "absolute inset-y-0 left-0 rounded-xs",
            flag.global ? "bg-k-ok" : "bg-primary",
          )}
          style={{ width: `${share}%` }}
        />
      </span>
      <span className="text-right font-mono tabular-nums">{flag.runs.toLocaleString("en")}</span>
      <span
        className={cn(
          flag.tone === "bad" && "text-destructive",
          flag.tone === "muted" && "text-muted-foreground",
        )}
      >
        {flag.note}
      </span>
    </>
  );
}

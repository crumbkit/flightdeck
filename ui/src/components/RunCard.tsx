import { cn } from "cn";
import { type ReactNode, useState } from "react";
import { notStartedWord } from "@/api/failure";
import type { Command, Link, Tool } from "@/api/model";
import { type ItemFailure, type Output, resultOf, rowsOf, streamReader } from "@/api/result";
import type { RunRequest } from "@/api/runs";
import { useNow, useSampled } from "@/clock";
import { elapsedWords } from "@/lib/duration";
import { needsConfirmation } from "@/lib/rail";
import { refusedSpan } from "@/lib/refusal";
import type { PageSize, View } from "@/lib/table";
import { copiedLine } from "@/lib/table";
import type { Run, RunState } from "@/lib/transcript";
import { CopyButton } from "./CopyButton";
import { FailureCard } from "./FailureCard";
import { NextChips } from "./NextChips";
import { RecordView } from "./RecordView";
import { ResultTable } from "./ResultTable";

type Props = {
  run: Run;
  tool: Tool;
  command: Command | undefined;
  /** The link that each `id` cell of the result opens, or null for none. */
  link: Link | null;
  /** The page's words for a cancel of this run that failed, or null. */
  cancelWords: string | null;
  onEdit: () => void;
  onRunAgain: () => void;
  onCancel: (id: string) => void;
  onPageSize: (size: PageSize) => void;
  onOpenLink: (value: string) => void;
  /** What follows the result, such as a preview's question. */
  children?: ReactNode;
  /** Fills the composer with a request, such as a failure's recovery, and runs nothing. */
  onCompose: (request: RunRequest) => void;
};

/** Returns the lines of standard output that a run has so far. */
function linesOf(state: RunState): string[] {
  return state.kind === "running" || state.kind === "ended" || state.kind === "cut"
    ? state.lines
    : [];
}

/** Returns "1 row" or "6 rows". */
function rowWords(count: number): string {
  return count === 1 ? "1 row" : `${count} rows`;
}

/**
 * The status of a run in words, and whether it failed. `rows` is the
 * number of rows its table holds, or null for a result that is no table.
 */
function statusOf(run: Run, rows: number | null): { words: string; failed: boolean } {
  const { state, ask } = run;
  if (ask?.answer === "declined") {
    return { words: "not confirmed", failed: false };
  }
  if (ask?.answer === "open" && ask.confirmation !== null) {
    return { words: "awaiting confirmation", failed: false };
  }
  const counted = rows === null ? "" : ` · ${rowWords(rows)}`;
  switch (state.kind) {
    case "starting":
    case "running":
      return { words: ask?.answer === "open" ? "previewing" : `running${counted}`, failed: false };
    case "issued":
      return { words: "awaiting confirmation", failed: false };
    case "refused":
      return { words: notStartedWord(state.problem), failed: true };
    case "broken":
      return { words: "no answer", failed: true };
    case "cut":
      return { words: "cut off", failed: true };
    case "ended": {
      const exit = state.exit === null ? "no exit code" : `exit ${state.exit}`;
      if (state.outcome.kind === "cancelled") {
        return { words: `cancelled · ${exit} · ${rowWords(rows ?? 0)} kept`, failed: false };
      }
      return { words: `${exit}${counted}`, failed: state.exit !== 0 };
    }
  }
}

function timeOf(iso: string | null): string | null {
  if (iso === null || iso === "") {
    return null;
  }
  const date = new Date(iso);
  return Number.isNaN(date.getTime())
    ? null
    : date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

/** The time since a run started, in tenths of a second, read ten times a second. */
function Elapsed({ startedAt }: { startedAt: string }) {
  const now = useNow(100, true);
  const started = Date.parse(startedAt);
  if (Number.isNaN(started)) {
    return null;
  }
  return <span>{(Math.max(0, now - started) / 1000).toFixed(1)} s</span>;
}

/**
 * The card's status for a screen reader: a polite live region that a
 * running card changes at most once a second, and that says the final
 * status once, when the run ends.
 */
function LiveStatus({ words, running }: { words: string; running: boolean }) {
  const said = useSampled(words, 1000, running);
  return (
    <span role="status" aria-live="polite" className="sr-only">
      {said}
    </span>
  );
}

/** A run's line, with the word that a refusal names underlined. */
function LineText({ line, state }: { line: string; state: RunState }) {
  let problem = null;
  if (state.kind === "refused") {
    problem = state.problem;
  } else if (state.kind === "ended" && state.outcome.kind === "tool_failure") {
    problem = state.outcome.failure;
  }
  const span = refusedSpan(line, problem);
  if (span === null) {
    return <code className="text-foreground font-mono">{line}</code>;
  }
  const [start, end] = span;
  return (
    <code className="text-foreground font-mono">
      {line.slice(0, start)}
      <span className="decoration-destructive underline decoration-wavy">
        {line.slice(start, end)}
      </span>
      {line.slice(end)}
    </code>
  );
}

/**
 * One run in the transcript: its line, start time, duration and exit, its
 * result, and Edit, Run again, Copy and a JSON view. A running card shows
 * the time since the run started, its rows so far and Cancel. A preview
 * card adds crumbkit's dry-run sentence above its rows, and its question
 * as `children`. A command that needs a confirmation has Preview again in
 * place of Run again.
 */
export function RunCard(props: Props) {
  const { run, command, cancelWords, onEdit, onRunAgain, onCancel, children } = props;
  const [view, setView] = useState<View>({ long: run.long, fields: null });
  const [json, setJson] = useState(false);
  const { state } = run;
  const [readStream] = useState(() => streamReader());
  const form = command?.result.form ?? null;
  const output = form === "stream" ? readStream(linesOf(state)) : resultOf(linesOf(state), form);
  const { result } = output;
  const rows = result.kind === "table" ? rowsOf(result.entries).length : null;
  const status = statusOf(run, rows);
  const running = state.kind === "starting" || state.kind === "running";
  const cancelled = state.kind === "ended" && state.outcome.kind === "cancelled";
  const startedAt =
    state.kind === "running" || state.kind === "ended" || state.kind === "cut"
      ? state.startedAt
      : null;
  const started = timeOf(startedAt);
  const line = copiedLine(run.line, view);

  return (
    <section
      id={`run-${run.number}`}
      aria-label={`Run ${run.number}: ${run.line}, ${status.words}`}
      className="bg-card mb-3 overflow-hidden rounded-lg border"
    >
      <header
        className={cn(
          "bg-muted text-muted-foreground flex flex-wrap items-center gap-x-3 gap-y-1 border-b px-3 py-1.5 text-xs",
          cancelled && "opacity-60",
        )}
      >
        <span className="text-foreground font-semibold">Run {run.number}</span>
        <LineText line={line} state={state} />
        {started !== null && <time dateTime={startedAt ?? undefined}>{started}</time>}
        {state.kind === "ended" && <span>{elapsedWords(state.durationMs)}</span>}
        {state.kind === "running" && <Elapsed startedAt={state.startedAt} />}
        <span className={cn("font-mono", status.failed ? "text-destructive" : "text-read-only")}>
          {status.words}
        </span>
        <LiveStatus words={status.words} running={running} />
        <span className="ml-auto flex items-center gap-3">
          {state.kind === "running" && state.id !== null && (
            <button
              type="button"
              className="text-primary"
              title="Cancel the run (⌘.)"
              onClick={() => state.id !== null && onCancel(state.id)}
            >
              Cancel
            </button>
          )}
          <button type="button" className="text-primary" onClick={onEdit}>
            Edit
          </button>
          <button type="button" className="text-primary" onClick={onRunAgain}>
            {command !== undefined && needsConfirmation(command) ? "Preview again" : "Run again"}
          </button>
          <CopyButton text={line} label="Copy line" />
          <label className="inline-flex items-center gap-1">
            <input
              type="checkbox"
              checked={json}
              onChange={(event) => setJson(event.target.checked)}
            />
            JSON
          </label>
        </span>
      </header>
      {running && (
        <div aria-hidden="true" className="bg-muted h-0.5 overflow-hidden">
          <div className="bg-primary h-full w-1/3 motion-safe:animate-pulse" />
        </div>
      )}
      <div className="px-3 py-2.5">
        {isDryRun(run) && (
          <p className="bg-muted mt-0 mb-2 rounded-md px-2.5 py-1.5">
            <span className="font-semibold">Dry run.</span> This was a dry run, so nothing changed.
          </p>
        )}
        {cancelWords !== null && (
          <p role="alert" className="mt-0">
            {cancelWords}
          </p>
        )}
        {json ? (
          <JsonView state={state} output={output} />
        ) : (
          <Body {...props} view={view} onView={setView} output={output} />
        )}
        {children}
      </div>
    </section>
  );
}

/** Returns true for a preview card whose dry run ended with a result. */
function isDryRun(run: Run): boolean {
  return (
    run.ask !== undefined &&
    run.ask.answer !== "confirmed" &&
    run.state.kind === "ended" &&
    run.state.outcome.kind === "result"
  );
}

function JsonView({ state, output }: { state: RunState; output: Output }) {
  const shown = output.document ?? (state.kind === "refused" ? state.problem : null);
  return (
    <pre className="bg-code max-h-96 overflow-auto rounded-md p-2.5 font-mono text-xs">
      {JSON.stringify(shown, null, 2)}
    </pre>
  );
}

type BodyProps = Props & { view: View; onView: (view: View) => void; output: Output };

function Body(props: BodyProps) {
  const { run, tool, command, onCompose, output } = props;
  const { state } = run;
  // The rows that a run printed before it stopped stay on the card.
  const kept = output.result.kind === "table" || output.result.kind === "record";
  switch (state.kind) {
    case "starting":
      return <p className="text-muted-foreground">Running…</p>;
    case "issued":
      return <p className="m-0">This command has no preview.</p>;
    case "running":
      return output.result.kind === "table" && output.result.entries.length > 0 ? (
        <Drawn {...props} />
      ) : (
        <p className="text-muted-foreground">Running…</p>
      );
    case "refused":
      return (
        <FailureCard
          failure={state.problem}
          words={state.words}
          heading={null}
          tool={tool}
          onCompose={onCompose}
        />
      );
    case "broken":
      return <p role="alert">{state.words}</p>;
    case "cut":
      return (
        <>
          {kept && <Drawn {...props} />}
          <p role="alert" className="mb-0">
            The run's output stopped before the run ended, so its result is incomplete.{" "}
            {state.words ??
              "Check that flightdeck open is still running in your terminal, then run it again."}
          </p>
        </>
      );
    case "ended": {
      const { outcome } = state;
      if (outcome.kind === "cancelled") {
        return (
          <>
            {kept && <Drawn {...props} />}
            <p className="text-muted-foreground mb-0">
              The run was cancelled, so its result is partial.
            </p>
          </>
        );
      }
      // A stream's own failure line is drawn in its place among its rows.
      const streamed = command?.result.form === "stream";
      if (outcome.kind === "failed" || (outcome.kind === "tool_failure" && !streamed)) {
        const { failure } = outcome;
        return (
          <FailureCard
            failure={failure}
            words={failure.message ?? failure.cause ?? "The run failed."}
            heading={null}
            tool={tool}
            onCompose={onCompose}
          />
        );
      }
      return (
        <>
          <Drawn {...props} />
          <NextChips steps={output.next} tool={tool} onCompose={onCompose} />
        </>
      );
    }
  }
}

/** The heading of a batch's failed item, such as "7 (index 1)". */
function headingOf(failure: ItemFailure): string | null {
  if (failure.key === undefined) {
    return null;
  }
  return failure.index === undefined ? failure.key : `${failure.key} (index ${failure.index})`;
}

/** A result as the page draws it: a table, a record, prose or a note. */
function Drawn(props: BodyProps) {
  const { run, tool, command, view, onView, onPageSize, onCompose, output, link, onOpenLink } =
    props;
  const { result } = output;
  const label = `Result of run ${run.number}`;
  const drawFailure = (failure: ItemFailure) => (
    <FailureCard
      failure={failure}
      words={failure.message}
      heading={headingOf(failure)}
      tool={tool}
      onCompose={onCompose}
    />
  );
  switch (result.kind) {
    case "table":
      return command === undefined ? null : (
        <ResultTable
          command={command}
          entries={result.entries}
          listing={result.listing}
          view={view}
          onView={onView}
          onPageSize={onPageSize}
          label={label}
          drawFailure={drawFailure}
          link={link}
          onOpenLink={onOpenLink}
        />
      );
    case "record":
      return (
        <div className="flex flex-col gap-2">
          {result.entries.map((entry, index) =>
            entry.kind === "row" ? (
              // A result's entries have no key of their own and never change order.
              // biome-ignore lint/suspicious/noArrayIndexKey: a result's entries are fixed
              <RecordView key={index} record={entry.row} label={label} />
            ) : (
              // biome-ignore lint/suspicious/noArrayIndexKey: a result's entries are fixed
              <div key={index}>{drawFailure(entry.failure)}</div>
            ),
          )}
        </div>
      );
    case "prose":
      return <p className="m-0 max-w-prose">{result.text}</p>;
    case "other":
      return <p className="text-muted-foreground">The result has no table. Open the JSON view.</p>;
    case "none":
      return <p className="text-muted-foreground">The command printed no result.</p>;
  }
}

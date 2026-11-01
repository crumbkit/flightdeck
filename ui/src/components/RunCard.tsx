import { cn } from "cn";
import { useState } from "react";
import type { Command } from "@/api/model";
import { resultOf } from "@/api/result";
import type { FailureDocument } from "@/api/runs";
import { elapsedWords } from "@/lib/duration";
import type { PageSize, View } from "@/lib/table";
import { copiedLine } from "@/lib/table";
import type { Run, RunState } from "@/lib/transcript";
import { CopyButton } from "./CopyButton";
import { RecordView } from "./RecordView";
import { ResultTable } from "./ResultTable";

type Props = {
  run: Run;
  command: Command | undefined;
  onEdit: () => void;
  onRunAgain: () => void;
  onPageSize: (size: PageSize) => void;
};

/** The status of a run as a word, and whether it failed. */
function statusOf(state: RunState): { words: string; failed: boolean } {
  switch (state.kind) {
    case "starting":
    case "running":
      return { words: "running", failed: false };
    case "refused":
      return { words: "refused", failed: true };
    case "broken":
      return { words: "no answer", failed: true };
    case "cut":
      return { words: "cut off", failed: true };
    case "ended":
      if (state.outcome.kind === "cancelled") {
        return { words: "cancelled", failed: false };
      }
      return {
        words: state.exit === null ? "no exit code" : `exit ${state.exit}`,
        failed: state.exit !== 0,
      };
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

/**
 * One run in the transcript: its line, start time, duration and exit, its
 * result, and Edit, Run again, Copy and a JSON view.
 */
export function RunCard({ run, command, onEdit, onRunAgain, onPageSize }: Props) {
  const [view, setView] = useState<View>({ long: false, fields: null });
  const [json, setJson] = useState(false);
  const { state } = run;
  const status = statusOf(state);
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
      <header className="bg-muted text-muted-foreground flex flex-wrap items-center gap-x-3 gap-y-1 border-b px-3 py-1.5 text-xs">
        <span className="text-foreground font-semibold">Run {run.number}</span>
        <code className="text-foreground font-mono">{line}</code>
        {started !== null && <time dateTime={startedAt ?? undefined}>{started}</time>}
        {state.kind === "ended" && <span>{elapsedWords(state.durationMs)}</span>}
        <span className={cn("font-mono", status.failed ? "text-destructive" : "text-read-only")}>
          {status.words}
        </span>
        <span className="ml-auto flex items-center gap-3">
          <button type="button" className="text-primary" onClick={onEdit}>
            Edit
          </button>
          <button type="button" className="text-primary" onClick={onRunAgain}>
            Run again
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
      <div className="px-3 py-2.5">
        {json ? (
          <JsonView run={run} />
        ) : (
          <Body run={run} command={command} view={view} onView={setView} onPageSize={onPageSize} />
        )}
      </div>
    </section>
  );
}

function JsonView({ run }: { run: Run }) {
  const { state } = run;
  const lines =
    state.kind === "running" || state.kind === "ended" || state.kind === "cut" ? state.lines : [];
  const { document } = resultOf(lines);
  const shown = document ?? (state.kind === "refused" ? state.problem : null);
  return (
    <pre className="bg-code max-h-96 overflow-auto rounded-md p-2.5 font-mono text-xs">
      {JSON.stringify(shown, null, 2)}
    </pre>
  );
}

type BodyProps = {
  run: Run;
  command: Command | undefined;
  view: View;
  onView: (view: View) => void;
  onPageSize: (size: PageSize) => void;
};

function Body({ run, command, view, onView, onPageSize }: BodyProps) {
  const { state } = run;
  switch (state.kind) {
    case "starting":
    case "running":
      return <p className="text-muted-foreground">Running…</p>;
    case "refused":
      return (
        <PlainFailure failure={{ code: state.problem?.code ?? "refused", cause: state.words }} />
      );
    case "broken":
      return <p role="alert">{state.words}</p>;
    case "cut":
      return (
        <p role="alert">
          The run's output stopped before the run ended, so its result is incomplete. Check that
          flightdeck open is still running in your terminal, then run it again.
        </p>
      );
    case "ended": {
      const { outcome } = state;
      if (outcome.kind === "tool_failure" || outcome.kind === "failed") {
        return <PlainFailure failure={outcome.failure} />;
      }
      if (outcome.kind === "cancelled") {
        return <p className="text-muted-foreground">The run was cancelled.</p>;
      }
      const { result } = resultOf(state.lines);
      const label = `Result of run ${run.number}`;
      switch (result.kind) {
        case "table":
          return (
            <>
              {command !== undefined && (
                <ResultTable
                  command={command}
                  rows={result.rows}
                  listing={result.listing}
                  view={view}
                  onView={onView}
                  onPageSize={onPageSize}
                  label={label}
                />
              )}
              <ItemFailures failures={result.failures} />
            </>
          );
        case "record":
          return (
            <>
              <RecordView record={result.record} label={label} />
              <ItemFailures failures={result.failures} />
            </>
          );
        case "prose":
          return <p className="m-0 max-w-prose">{result.text}</p>;
        case "other":
          return (
            <p className="text-muted-foreground">The result has no table. Open the JSON view.</p>
          );
        case "none":
          return <p className="text-muted-foreground">The command printed no result.</p>;
      }
    }
  }
}

/** A failure document as a plain card with its code and cause. */
function PlainFailure({ failure }: { failure: FailureDocument }) {
  return (
    <div role="alert" className="border-l-destructive rounded-md border border-l-4 px-3 py-2">
      <p className="m-0 font-semibold">{failure.cause ?? failure.message ?? "The run failed."}</p>
      <code className="bg-code text-muted-foreground mt-1 inline-block rounded px-1.5 font-mono text-xs">
        {failure.code}
      </code>
    </div>
  );
}

/** The failed items of a batch, each with its key, message and code. */
function ItemFailures({
  failures,
}: {
  failures: { key?: string | undefined; code: string; message: string }[];
}) {
  if (failures.length === 0) {
    return null;
  }
  return (
    <ul className="mt-2 flex flex-col gap-1 p-0">
      {failures.map((failure) => (
        <li key={`${failure.key ?? ""}${failure.code}`} className="list-none">
          <PlainFailure
            failure={{
              code: failure.code,
              cause:
                failure.key === undefined ? failure.message : `${failure.key}: ${failure.message}`,
            }}
          />
        </li>
      ))}
    </ul>
  );
}

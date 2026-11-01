import { useMutation } from "@tanstack/react-query";
import { useEffect, useReducer, useRef, useState } from "react";
import { ApiError } from "@/api/client";
import { describeFailure } from "@/api/failure";
import type { Command, Example, ModelReply } from "@/api/model";
import { type RunRequest, runMutation } from "@/api/runs";
import { Composer, type FieldError } from "@/components/Composer";
import { Palette } from "@/components/Palette";
import { Rail } from "@/components/Rail";
import { RunCard } from "@/components/RunCard";
import { Topbar } from "@/components/Topbar";
import { Transcript } from "@/components/Transcript";
import { Welcome } from "@/components/Welcome";
import { requestOfExample } from "@/lib/example";
import { type Draft, draftFrom, emptyDraft, lineOf, missing, requestOf } from "@/lib/form";
import { whyNotRunnable } from "@/lib/rail";
import { withPageSize } from "@/lib/table";
import { transcript } from "@/lib/transcript";

/** The composer's command, what it holds, and the run it edits, if any. */
type Composing = { command: Command; draft: Draft; editing: number | null };

/**
 * The page of one tool: the rail, the transcript of runs with the welcome
 * card first, the composer docked at the bottom, and the ⌘K palette.
 */
export function ToolPage({ reply }: { reply: ModelReply }) {
  const { tool } = reply;
  const [runs, apply] = useReducer(transcript, []);
  const [composing, setComposing] = useState<Composing | null>(null);
  const [fieldError, setFieldError] = useState<FieldError | null>(null);
  const [finding, setFinding] = useState(false);
  const counter = useRef(0);
  const mutation = useMutation(runMutation());

  // ⌘K or Ctrl-K opens the palette from anywhere on the page.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key.toLowerCase() === "k" && (event.metaKey || event.ctrlKey)) {
        event.preventDefault();
        setFinding(true);
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, []);

  const commandAt = (request: RunRequest) =>
    tool.commands.find((each) => each.path.join(" ") === request.path.join(" "));

  const start = (request: RunRequest) => {
    const command = commandAt(request);
    if (command === undefined) {
      return;
    }
    counter.current += 1;
    const number = counter.current;
    setFieldError(null);
    apply({ type: "add", number, request, line: lineOf(tool.name, command, request) });
    mutation
      .mutateAsync({
        request,
        onEvent: (event) => apply({ type: "event", number, event }),
        signal: new AbortController().signal,
      })
      .catch((error: unknown) => {
        const words = describeFailure(error);
        if (error instanceof ApiError && error.failure.kind === "status") {
          const { problem } = error.failure;
          apply({ type: "refused", number, problem, words });
          const argument = problem?.fields?.argument;
          if (typeof argument === "string") {
            setFieldError({ name: argument, words });
          }
        } else {
          apply({ type: "broken", number, words });
        }
      });
  };

  const choose = (command: Command, draft: Draft = emptyDraft(command)) => {
    setComposing({ command, draft, editing: null });
    setFieldError(null);
  };

  const runExample = (command: Command, example: Example) => {
    const request = requestOfExample(tool, example.command);
    if (request === null) {
      choose(command);
      return;
    }
    const draft = draftFrom(command, request);
    choose(command, draft);
    if (whyNotRunnable(command, reply.read_only) === null && missing(command, draft).length === 0) {
      start(request);
    }
  };

  const edit = (number: number) => {
    const run = runs.find((each) => each.number === number);
    const command = run === undefined ? undefined : commandAt(run.request);
    if (run !== undefined && command !== undefined) {
      setComposing({ command, draft: draftFrom(command, run.request), editing: number });
    }
  };

  const showRun = (number: number) => {
    document.getElementById(`run-${number}`)?.scrollIntoView({ block: "nearest" });
  };

  const size = runs.reduce(
    (total, run) =>
      total +
      1 +
      (run.state.kind === "running" || run.state.kind === "ended" ? run.state.lines.length : 0),
    0,
  );

  return (
    <div className="flex h-full flex-col">
      <Topbar
        name={tool.name}
        version={tool.version}
        readOnly={reply.read_only}
        onFind={() => setFinding(true)}
      />
      <div className="flex min-h-0 flex-1">
        <Rail
          tool={tool}
          selected={composing?.command ?? null}
          onChoose={(command) => choose(command)}
        />
        <main className="flex min-w-0 flex-1 flex-col">
          <Transcript size={size}>
            <Welcome tool={tool} onExample={runExample} />
            {runs.map((run) => (
              <RunCard
                key={run.number}
                run={run}
                command={commandAt(run.request)}
                onEdit={() => edit(run.number)}
                onRunAgain={() => start(run.request)}
                onPageSize={(pageSize) => start(withPageSize(run.request, pageSize))}
              />
            ))}
          </Transcript>
          {composing === null ? (
            <p className="bg-card text-muted-foreground m-0 border-t px-4 py-3">
              Choose a command from the rail, or press ⌘K to find one.
            </p>
          ) : (
            <Composer
              command={composing.command}
              draft={composing.draft}
              editing={composing.editing}
              readOnly={reply.read_only}
              fieldError={fieldError}
              onChange={(draft) => setComposing({ ...composing, draft })}
              onRun={() => {
                setComposing({ ...composing, editing: null });
                start(requestOf(composing.command, composing.draft));
              }}
              onStopEditing={() => setComposing({ ...composing, editing: null })}
              onShowRun={showRun}
            />
          )}
        </main>
      </div>
      {finding && (
        <Palette
          tool={tool}
          onClose={() => setFinding(false)}
          onOpen={(command, example) => {
            const request = example === null ? null : requestOfExample(tool, example.command);
            choose(command, request === null ? emptyDraft(command) : draftFrom(command, request));
            setFinding(false);
          }}
        />
      )}
    </div>
  );
}

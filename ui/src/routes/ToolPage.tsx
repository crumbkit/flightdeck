import { useMutation } from "@tanstack/react-query";
import { cn } from "cn";
import { useEffect, useReducer, useRef, useState } from "react";
import { ApiError } from "@/api/client";
import { describeFailure, fieldErrorOf, isSignedOut } from "@/api/failure";
import type { Command, Example, Link, ModelReply } from "@/api/model";
import {
  cancelRun,
  type RunCall,
  type RunEvent,
  type RunRequest,
  streamConfirmation,
  streamPreview,
  streamRun,
  type Value,
} from "@/api/runs";
import { frameQueue } from "@/clock";
import { Composer, type FieldError } from "@/components/Composer";
import { Confirmation } from "@/components/Confirmation";
import { DirectoryControl } from "@/components/DirectoryControl";
import { LineEditor, shownLine, type Typed } from "@/components/LineEditor";
import { Palette } from "@/components/Palette";
import { Rail } from "@/components/Rail";
import { RunCard } from "@/components/RunCard";
import { Topbar } from "@/components/Topbar";
import { Transcript } from "@/components/Transcript";
import { Welcome } from "@/components/Welcome";
import { asksForYes } from "@/lib/confirm";
import { requestOfExample } from "@/lib/example";
import {
  type Draft,
  draftFrom,
  emptyDraft,
  lineOf,
  longOf,
  missing,
  requestOf,
  setValue,
} from "@/lib/form";
import { textOf } from "@/lib/line";
import { cellLinkOf, filled } from "@/lib/links";
import { needsConfirmation, whyNotRunnable } from "@/lib/rail";
import { withPageSize } from "@/lib/table";
import { drawnKey, type Run, transcript } from "@/lib/transcript";

/**
 * The composer's command, what it holds, the run it edits, if any, and a
 * number that changes each time the composer should take focus.
 */
type Composing = { command: Command; draft: Draft; editing: number | null; focus: number | null };

/** Returns the page's words for a failed cancel, or null when the run had already ended. */
function cancelFailure(error: unknown): string | null {
  const ended =
    error instanceof ApiError &&
    error.failure.kind === "status" &&
    error.failure.problem?.code === "flightdeck::run::not_running";
  return ended ? null : describeFailure(error);
}

type Props = {
  reply: ModelReply;
  /** Called when the server refuses the page's token during a run. */
  onSignedOut: () => void;
};

/**
 * The page of one tool: the rail, the transcript of runs with the welcome
 * card first, the composer docked at the bottom, and the ⌘K palette.
 */
export function ToolPage({ reply, onSignedOut }: Props) {
  const { tool } = reply;
  const [runs, apply] = useReducer(transcript, []);
  const [composing, setComposing] = useState<Composing | null>(null);
  const [fieldError, setFieldError] = useState<FieldError | null>(null);
  const [typed, setTyped] = useState<Typed | null>(null);
  const [finding, setFinding] = useState(false);
  const [directory, setDirectory] = useState(reply.directory);
  const counter = useRef(0);
  const focuses = useRef(0);
  // Each running run's controller, by its number. Leaving the page aborts
  // every run, which closes its stream, and the server cancels the run.
  const controllers = useRef(new Map<number, AbortController>());

  useEffect(() => {
    const running = controllers.current;
    return () => {
      for (const controller of running.values()) {
        controller.abort();
      }
      running.clear();
    };
  }, []);

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

  // The page holds each streamed run in the transcript's reducer, and a
  // cancel in a mutation (APP-11 in typescript-standards.toml).
  const cancel = useMutation({ mutationFn: cancelRun });
  const { mutate: cancelById } = cancel;

  // ⌘. or Ctrl-. cancels the newest running run, as in a terminal. Esc never
  // cancels, because it also closes the palette, and a stray Esc must not
  // stop a run.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "." || !(event.metaKey || event.ctrlKey)) {
        return;
      }
      const newest = runs.findLast((run) => run.state.kind === "running" && run.state.id !== null);
      if (newest?.state.kind === "running" && newest.state.id !== null) {
        event.preventDefault();
        cancelById(newest.state.id);
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [runs, cancelById]);

  const commandAt = (request: RunRequest) =>
    tool.commands.find((each) => each.path.join(" ") === request.path.join(" "));

  /** Returns `request` in the directory that the top bar shows as Run is pressed. */
  const inShownDirectory = (request: RunRequest): RunRequest =>
    directory === null ? request : { ...request, directory };

  /**
   * Streams the events of `open` into card `number`, and closes the card
   * when the stream ends. A refused request marks the card, and marks a
   * composer field when the refusal names one. A refused token signs the
   * page out. It resolves once the stream has ended either way.
   */
  const follow = (number: number, open: (call: Omit<RunCall, "request">) => Promise<void>) => {
    const controller = new AbortController();
    controllers.current.set(number, controller);
    // The events of one frame reach the transcript together, so a fast
    // stream draws once per frame. A tool's refusal of the line marks the
    // composer's field, as the server's does.
    const queue = frameQueue<RunEvent>((events) => {
      apply({ type: "events", number, events });
      for (const event of events) {
        if (event.event === "ended" && event.outcome.kind === "tool_failure") {
          const { failure } = event.outcome;
          const marked = fieldErrorOf(failure, failure.message ?? "");
          if (marked !== null) {
            setFieldError(marked);
          }
        }
      }
    });
    return open({ onEvent: queue.push, signal: controller.signal })
      .then(() => {
        queue.flush();
        apply({ type: "closed", number });
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) {
          return;
        }
        queue.flush();
        if (isSignedOut(error)) {
          onSignedOut();
          return;
        }
        const words = describeFailure(error);
        if (error instanceof ApiError && error.failure.kind === "status") {
          const { problem } = error.failure;
          apply({ type: "refused", number, problem, words });
          const marked = fieldErrorOf(problem, words);
          if (marked !== null) {
            setFieldError(marked);
          }
        } else {
          apply({ type: "broken", number, words });
        }
      })
      .finally(() => controllers.current.delete(number));
  };

  /** Returns the number of a new card. */
  const nextNumber = () => {
    counter.current += 1;
    setFieldError(null);
    return counter.current;
  };

  /**
   * Runs `request` in a new card. When the run ends with crumbkit's
   * needs_yes failure, the page opens a preview card for the same request,
   * so the person can confirm it.
   */
  const start = (request: RunRequest, long: boolean) => {
    const command = commandAt(request);
    if (command === undefined) {
      return;
    }
    const number = nextNumber();
    apply({ type: "add", number, request, line: lineOf(tool.name, command, request), long });
    let needsYes = false;
    void follow(number, (call) =>
      streamRun({
        ...call,
        request,
        onEvent: (event) => {
          needsYes ||= asksForYes(event);
          call.onEvent(event);
        },
      }),
    ).then(() => {
      if (needsYes) {
        preview(request, long, null);
      }
    });
  };

  /**
   * Previews `request` in a new card, which asks its question once the
   * server issues a confirmation id. `basis` is the composer's draft it was
   * made from, or null.
   */
  const preview = (request: RunRequest, long: boolean, basis: Draft | null) => {
    const command = commandAt(request);
    if (command === undefined) {
      return;
    }
    const number = nextNumber();
    const line = lineOf(tool.name, command, request);
    apply({ type: "preview", number, request, line, long, basis });
    void follow(number, (call) => streamPreview({ ...call, request }));
  };

  /**
   * Runs `request` at once, or previews it when it needs a confirmation.
   * `long` asks for the long columns.
   */
  const begin = (request: RunRequest, long: boolean, basis: Draft | null) => {
    const command = commandAt(request);
    if (command !== undefined && needsConfirmation(command)) {
      preview(request, long, basis);
    } else {
      start(request, long);
    }
  };

  /**
   * Runs a preview card's request with its confirmation id, once, in the
   * same card. The server refuses an id that is stale, used or issued for
   * another request, and the card then shows that failure.
   */
  const confirm = (run: Run) => {
    const id = run.ask?.confirmation;
    if (run.ask?.answer !== "open" || id === undefined || id === null) {
      return;
    }
    apply({ type: "answer", number: run.number, answer: "confirmed" });
    void follow(run.number, (call) => streamConfirmation(id, { ...call, request: run.request }));
  };

  const choose = (command: Command, draft: Draft = emptyDraft(command)) => {
    setComposing({ command, draft, editing: null, focus: null });
    setFieldError(null);
  };

  const nextFocus = () => {
    focuses.current += 1;
    return focuses.current;
  };

  // A chip fills the composer and moves focus to it, and runs nothing.
  const compose = (request: RunRequest) => {
    const command = commandAt(request);
    if (command !== undefined) {
      setComposing({
        command,
        draft: draftFrom(command, request),
        editing: null,
        focus: nextFocus(),
      });
      setFieldError(null);
    }
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
      begin(inShownDirectory(request), false, draft);
    }
  };

  /**
   * Returns the composer filled from the line's command and values. A
   * switch keeps the alternative it was on when the line gives neither.
   */
  const fromLine = (command: Command, values: Record<string, Value>): Composing => {
    const draft = draftFrom(command, { path: command.path, values });
    if (composing === null || composing.command !== command) {
      return { command, draft, editing: null, focus: null };
    }
    return {
      ...composing,
      draft: { ...draft, chosen: { ...composing.draft.chosen, ...draft.chosen } },
    };
  };

  /** Runs what the composer holds, when the command can run and has every required value. */
  const run = (next: Composing) => {
    setComposing({ ...next, editing: null });
    const { command, draft } = next;
    if (whyNotRunnable(command, reply.read_only) === null && missing(command, draft).length === 0) {
      begin(inShownDirectory(requestOf(command, draft)), longOf(draft), draft);
    }
  };

  const edit = (number: number) => {
    const run = runs.find((each) => each.number === number);
    const command = run === undefined ? undefined : commandAt(run.request);
    if (run !== undefined && command !== undefined) {
      const draft = setValue(draftFrom(command, run.request), "--long", run.long ? true : null);
      setComposing({ command, draft, editing: number, focus: nextFocus() });
    }
  };

  // An inferred link fills the target's composer and runs nothing.
  const openLink = (link: Link, value: string) => {
    const target = tool.commands.find((each) => each.path.join(" ") === link.target.join(" "));
    if (target !== undefined) {
      choose(target, filled(target, link.argument, value));
    }
  };

  const showRun = (number: number) => {
    document.getElementById(`run-${number}`)?.scrollIntoView({ block: "nearest" });
  };

  const canonical = composing === null ? "" : textOf(tool, composing.command, composing.draft);
  const lineRefused = shownLine(tool, typed, canonical).parsed.problem !== null;
  const line = (
    <LineEditor
      tool={tool}
      canonical={canonical}
      typed={typed}
      onTyped={setTyped}
      onApply={(command, values) => setComposing(fromLine(command, values))}
      onRun={(command, values) => run(fromLine(command, values))}
    />
  );

  return (
    <div className="flex h-full flex-col">
      <Topbar
        name={tool.name}
        version={tool.version}
        readOnly={reply.read_only}
        onFind={() => setFinding(true)}
      >
        <DirectoryControl directory={directory} onChange={setDirectory} />
      </Topbar>
      <div className="flex min-h-0 flex-1">
        <Rail
          tool={tool}
          selected={composing?.command ?? null}
          readOnly={reply.read_only}
          onChoose={(command) => choose(command)}
        />
        <main className="flex min-w-0 flex-1 flex-col">
          <Transcript
            content={drawnKey(runs)}
            latest={runs.length === 0 ? null : `run-${runs[runs.length - 1]?.number}`}
          >
            <Welcome tool={tool} onExample={runExample} />
            {runs.map((run) => {
              const command = commandAt(run.request);
              const link = command === undefined ? null : cellLinkOf(tool, command);
              return (
                <RunCard
                  key={run.number}
                  run={run}
                  tool={tool}
                  command={command}
                  link={link}
                  cancelWords={
                    cancel.isError &&
                    run.state.kind === "running" &&
                    cancel.variables === run.state.id
                      ? cancelFailure(cancel.error)
                      : null
                  }
                  onEdit={() => edit(run.number)}
                  onRunAgain={() => begin(run.request, run.long, null)}
                  onCancel={cancelById}
                  onPageSize={(pageSize) =>
                    begin(withPageSize(run.request, pageSize), run.long, null)
                  }
                  onCompose={compose}
                  onOpenLink={(value) => link !== null && openLink(link, value)}
                >
                  {command !== undefined && (
                    <Confirmation
                      run={run}
                      command={command}
                      stale={
                        run.ask !== undefined &&
                        run.ask.basis !== null &&
                        run.ask.basis !== composing?.draft
                      }
                      onConfirm={() => confirm(run)}
                      onDecline={() =>
                        apply({ type: "answer", number: run.number, answer: "declined" })
                      }
                    />
                  )}
                </RunCard>
              );
            })}
          </Transcript>
          {/* The line stays in one place in the page, so it keeps its focus
              when the first command it names opens the form. */}
          <div
            className={cn(
              "bg-card sticky bottom-0 flex shrink-0 flex-col border-t px-4 py-3",
              composing === null || composing.command.cost === null
                ? "max-h-dock"
                : "max-h-dock-costed",
            )}
          >
            {line}
            {composing === null ? (
              <p className="text-muted-foreground m-0 text-xs">
                Type a line, <span className="hidden md:inline">pick a command in the rail, </span>
                or <span className="md:hidden">tap Find a command</span>
                <span className="hidden md:inline">press ⌘K</span>.
              </p>
            ) : (
              <Composer
                tool={tool}
                command={composing.command}
                draft={composing.draft}
                editing={composing.editing}
                focus={composing.focus}
                readOnly={reply.read_only}
                fieldError={fieldError}
                lineRefused={lineRefused}
                directory={directory}
                onChange={(draft) => setComposing({ ...composing, draft })}
                onRun={() => run(composing)}
                onStopEditing={() => setComposing({ ...composing, editing: null })}
                onShowRun={showRun}
              />
            )}
          </div>
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

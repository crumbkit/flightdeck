import { useEffect, useReducer, useRef, useState } from "react";
import { ApiError } from "@/api/client";
import { describeFailure, fieldErrorOf, isSignedOut } from "@/api/failure";
import type { Command, Example, ModelReply } from "@/api/model";
import {
  type RunCall,
  type RunRequest,
  streamConfirmation,
  streamPreview,
  streamRun,
} from "@/api/runs";
import { Composer, type FieldError } from "@/components/Composer";
import { Confirmation } from "@/components/Confirmation";
import { Palette } from "@/components/Palette";
import { Rail } from "@/components/Rail";
import { RunCard } from "@/components/RunCard";
import { Topbar } from "@/components/Topbar";
import { Transcript } from "@/components/Transcript";
import { Welcome } from "@/components/Welcome";
import { asksForYes } from "@/lib/confirm";
import { requestOfExample } from "@/lib/example";
import { type Draft, draftFrom, emptyDraft, lineOf, missing, requestOf } from "@/lib/form";
import { needsConfirmation, whyNotRunnable } from "@/lib/rail";
import { withPageSize } from "@/lib/table";
import { type Run, transcript } from "@/lib/transcript";

/** The composer's command, what it holds, and the run it edits, if any. */
type Composing = { command: Command; draft: Draft; editing: number | null };

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
  const [finding, setFinding] = useState(false);
  const counter = useRef(0);
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

  const commandAt = (request: RunRequest) =>
    tool.commands.find((each) => each.path.join(" ") === request.path.join(" "));

  /**
   * Streams the events of `open` into card `number`, and closes the card
   * when the stream ends. A refused request marks the card, and marks a
   * composer field when the refusal names one. A refused token signs the
   * page out. It resolves once the stream has ended either way.
   */
  const follow = (number: number, open: (call: Omit<RunCall, "request">) => Promise<void>) => {
    const controller = new AbortController();
    controllers.current.set(number, controller);
    return open({
      onEvent: (event) => apply({ type: "event", number, event }),
      signal: controller.signal,
    })
      .then(() => apply({ type: "closed", number }))
      .catch((error: unknown) => {
        if (controller.signal.aborted) {
          return;
        }
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
  const start = (request: RunRequest) => {
    const command = commandAt(request);
    if (command === undefined) {
      return;
    }
    const number = nextNumber();
    apply({ type: "add", number, request, line: lineOf(tool.name, command, request) });
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
        preview(request, null);
      }
    });
  };

  /**
   * Previews `request` in a new card, which asks its question once the
   * server issues a confirmation id. `basis` is the composer's draft it was
   * made from, or null.
   */
  const preview = (request: RunRequest, basis: Draft | null) => {
    const command = commandAt(request);
    if (command === undefined) {
      return;
    }
    const number = nextNumber();
    apply({ type: "preview", number, request, line: lineOf(tool.name, command, request), basis });
    void follow(number, (call) => streamPreview({ ...call, request }));
  };

  /** Runs `request` at once, or previews it when it needs a confirmation. */
  const begin = (request: RunRequest, basis: Draft | null) => {
    const command = commandAt(request);
    if (command !== undefined && needsConfirmation(command)) {
      preview(request, basis);
    } else {
      start(request);
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
      begin(request, draft);
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
          readOnly={reply.read_only}
          onChoose={(command) => choose(command)}
        />
        <main className="flex min-w-0 flex-1 flex-col">
          <Transcript size={size}>
            <Welcome tool={tool} onExample={runExample} />
            {runs.map((run) => {
              const command = commandAt(run.request);
              return (
                <RunCard
                  key={run.number}
                  run={run}
                  command={command}
                  onEdit={() => edit(run.number)}
                  onRunAgain={() => begin(run.request, null)}
                  onPageSize={(pageSize) => begin(withPageSize(run.request, pageSize), null)}
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
                begin(requestOf(composing.command, composing.draft), composing.draft);
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

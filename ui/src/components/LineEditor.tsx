import { cn } from "cn";
import {
  type ChangeEvent,
  type KeyboardEvent,
  type ReactNode,
  useId,
  useLayoutEffect,
  useRef,
} from "react";
import type { Command, Tool } from "@/api/model";
import type { Value } from "@/api/runs";
import { draftFrom } from "@/lib/form";
import { type Mark, notesOf, type Parsed, parseLine, stripPrompt, textOf } from "@/lib/line";

/**
 * What the person typed in the line: the text, the caret while the line
 * has focus or null after they leave it, the notes about flags that were
 * taken out, and the form's line when they typed it. The form's line
 * replaces the typed text once the line has no focus and the form changes.
 */
export type Typed = { text: string; caret: number | null; base: string; notes: string[] };

/** The line as the page shows it: its text, what it says and the notes under it. */
export type Shown = { text: string; parsed: Parsed; notes: string[] };

/**
 * Returns the line the page shows: what the person typed while the line
 * has focus or the form has not changed since, and otherwise `canonical`,
 * the form's own line.
 */
export function shownLine(tool: Tool, typed: Typed | null, canonical: string): Shown {
  if (typed === null || (typed.caret === null && typed.base !== canonical)) {
    return { text: canonical, parsed: parseLine(tool, canonical, null), notes: [] };
  }
  return { text: typed.text, parsed: parseLine(tool, typed.text, typed.caret), notes: typed.notes };
}

type Props = {
  tool: Tool;
  /** The form's line, without the tool's name, or "" when no command is chosen. */
  canonical: string;
  typed: Typed | null;
  onTyped: (typed: Typed | null) => void;
  /** Fills the form from a line that names a command and has no problem. */
  onApply: (command: Command, values: Record<string, Value>) => void;
  /** Fills the form and runs it, for ↩ or ⌘↩ in the line. */
  onRun: (command: Command, values: Record<string, Value>) => void;
};

/** The colour of each kind of word in the line. */
const colours: Record<Mark["kind"], string> = {
  path: "text-foreground",
  flag: "text-primary",
  value: "text-foreground",
  positional: "text-read-only",
};

/**
 * The command line above the form. While the line has focus it is the
 * source: each change fills the form from the words the person has
 * finished, and the text stays exactly as typed. When the person leaves
 * the line, it is rewritten in the form's order. A word the tool would
 * refuse is underlined, with the tool's words and a fix under the line.
 *
 * The text is drawn twice: a transparent input takes the typing and the
 * caret, and a copy behind it draws the colours.
 */
export function LineEditor({ tool, canonical, typed, onTyped, onApply, onRun }: Props) {
  const input = useRef<HTMLInputElement>(null);
  const behind = useRef<HTMLDivElement>(null);
  const caretAfterRender = useRef<number | null>(null);
  const messageId = useId();
  const { text, parsed, notes } = shownLine(tool, typed, canonical);
  const { problem } = parsed;
  const fix = problem?.fix ?? null;

  // A dropped prefix moves the caret back by its length, after React has
  // written the shorter text. The copy behind follows the input's scroll.
  useLayoutEffect(() => {
    const field = input.current;
    if (field === null) {
      return;
    }
    if (caretAfterRender.current !== null) {
      field.setSelectionRange(caretAfterRender.current, caretAfterRender.current);
      caretAfterRender.current = null;
    }
    if (behind.current !== null) {
      behind.current.scrollLeft = field.scrollLeft;
    }
  });

  /** Reads the line with the caret at `caret` and fills the form when it can. */
  const edit = (next: string, caret: number) => {
    const line = parseLine(tool, next, caret);
    onTyped({ text: next, caret, base: canonical, notes: notesOf(line.dropped) });
    if (!line.pending && line.problem === null && line.command !== null) {
      onApply(line.command, line.values);
    }
  };

  /**
   * Reads the whole line as finished. A line with no problem fills the form
   * and is rewritten in the form's order, an empty line shows the form's
   * line again, and a refused line stays as typed.
   */
  const finish = (
    next: string,
    then: (command: Command, values: Record<string, Value>) => void,
  ) => {
    const line = parseLine(tool, next, null);
    const { command, values } = line;
    if (command !== null && line.problem === null) {
      then(command, values);
      const rewritten = textOf(tool, command, draftFrom(command, { path: command.path, values }));
      onTyped({ text: rewritten, caret: null, base: rewritten, notes: notesOf(line.dropped) });
    } else if (next.trim() === "") {
      onTyped(null);
    } else {
      onTyped({ text: next, caret: null, base: canonical, notes: notesOf(line.dropped) });
    }
  };

  const onChange = (event: ChangeEvent<HTMLInputElement>) => {
    const { value, selectionStart } = event.target;
    const next = stripPrompt(tool, value);
    const removed = value.length - next.length;
    const caret = Math.max(0, (selectionStart ?? value.length) - removed);
    if (removed > 0) {
      caretAfterRender.current = caret;
    }
    edit(next, caret);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      finish(text, onRun);
    }
  };

  return (
    <div className="mb-2.5 flex flex-col gap-1">
      <div
        className={cn(
          // The input draws no ring of its own, so the box around it shows
          // the page's 3 px focus ring.
          "bg-code focus-within:outline-ring flex min-w-0 items-center gap-2.5 rounded-lg border px-3 py-2 font-mono text-xs focus-within:outline-3 focus-within:outline-offset-1 focus-within:outline-solid",
          problem !== null && "border-destructive",
        )}
      >
        <span aria-hidden="true" className="text-faint-foreground">
          $
        </span>
        <span aria-hidden="true">{tool.name}</span>
        <div className="relative min-w-0 flex-1">
          <div
            ref={behind}
            aria-hidden="true"
            className="pointer-events-none absolute inset-0 flex items-center overflow-hidden whitespace-pre"
          >
            {drawn(text, parsed)}
          </div>
          <input
            ref={input}
            type="text"
            aria-label={`Command line, after ${tool.name}`}
            aria-invalid={problem === null ? undefined : true}
            aria-describedby={problem === null ? undefined : messageId}
            autoCapitalize="off"
            autoComplete="off"
            autoCorrect="off"
            spellCheck={false}
            placeholder={tool.commands[0]?.path.join(" ")}
            value={text}
            onChange={onChange}
            onSelect={(event) => {
              const caret = event.currentTarget.selectionStart;
              if (
                typed !== null &&
                typed.caret !== null &&
                caret !== null &&
                caret !== typed.caret
              ) {
                edit(text, caret);
              }
            }}
            onFocus={(event) => edit(text, event.currentTarget.selectionStart ?? text.length)}
            onBlur={() => finish(text, onApply)}
            onKeyDown={onKeyDown}
            onScroll={(event) => {
              if (behind.current !== null) {
                behind.current.scrollLeft = event.currentTarget.scrollLeft;
              }
            }}
            className="caret-foreground placeholder:text-faint-foreground relative w-full bg-transparent text-transparent outline-none"
          />
        </div>
      </div>
      {problem !== null && (
        <div className="text-destructive flex flex-wrap items-baseline gap-2 text-xs">
          <span id={messageId}>{problem.words}</span>
          {fix !== null && (
            <button
              type="button"
              className="border-destructive bg-card rounded-full border px-2"
              onClick={() => {
                const { start, end, text: replacement } = fix;
                finish(text.slice(0, start) + replacement + text.slice(end), onApply);
              }}
            >
              Change to {fix.text}
            </button>
          )}
        </div>
      )}
      {notes.map((note) => (
        <p key={note} className="text-faint-foreground m-0 text-xs">
          {note}
        </p>
      ))}
    </div>
  );
}

/**
 * Draws the line's words in their colours, quoted words muted, and the
 * refused part with a wavy underline. The text is cut wherever a word or
 * the refused part starts or ends, and each piece takes the colour of the
 * word it is in.
 */
function drawn(text: string, parsed: Parsed): ReactNode[] {
  const { marks, problem } = parsed;
  const cuts = new Set([0, text.length]);
  for (const mark of marks) {
    cuts.add(mark.start);
    cuts.add(mark.end);
  }
  if (problem !== null) {
    cuts.add(problem.start);
    cuts.add(problem.end);
  }
  const points = [...cuts].sort((a, b) => a - b);
  return points.slice(1).map((end, index) => {
    const start = points[index] ?? 0;
    const mark = marks.find((each) => each.start <= start && end <= each.end);
    const refused = problem !== null && problem.start <= start && end <= problem.end;
    return (
      <span
        key={start}
        className={cn(
          mark !== undefined && (mark.quoted ? "text-muted-foreground" : colours[mark.kind]),
          refused && "decoration-destructive underline decoration-wavy underline-offset-3",
        )}
      >
        {text.slice(start, end)}
      </span>
    );
  });
}

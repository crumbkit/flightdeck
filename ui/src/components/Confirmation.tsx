import { type KeyboardEvent, useEffect, useRef } from "react";
import type { Command } from "@/api/model";
import { Button } from "@/components/ui/button";
import { confirmedLine, confirmLabel, countOf, effectsInWords, question } from "@/lib/confirm";
import { effectOf } from "@/lib/rail";
import type { Run } from "@/lib/transcript";
import { EffectBadge } from "./EffectBadge";

type Props = {
  run: Run;
  command: Command;
  /** True when the composer has changed since the preview. */
  stale: boolean;
  onConfirm: () => void;
  onDecline: () => void;
};

/**
 * The end of a preview card: once the server has issued its confirmation
 * id, the question with the verb and the count, the effects in words, the
 * exact line that will run, and Cancel and the red confirm button. Cancel
 * takes focus, so Enter cancels, and ⌘↩ never confirms. A stale preview
 * disables its confirm button. After Cancel the card says that nothing
 * changed.
 */
export function Confirmation({ run, command, stale, onConfirm, onDecline }: Props) {
  const { ask, state } = run;
  if (ask === undefined || ask.answer === "confirmed") {
    return null;
  }
  if (ask.answer === "declined") {
    return <p className="text-muted-foreground mt-2 mb-0">Not confirmed, so nothing changed.</p>;
  }
  if (ask.confirmation === null) {
    return null;
  }
  const count = state.kind === "ended" ? countOf(state.lines) : null;
  return (
    <Question
      command={command}
      count={count}
      line={confirmedLine(command, run.line)}
      stale={stale}
      onConfirm={onConfirm}
      onDecline={onDecline}
    />
  );
}

type QuestionProps = {
  command: Command;
  count: number | null;
  line: string;
  stale: boolean;
  onConfirm: () => void;
  onDecline: () => void;
};

function Question({ command, count, line, stale, onConfirm, onDecline }: QuestionProps) {
  const cancel = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    cancel.current?.focus();
  }, []);

  // A focused button presses itself on Enter whatever the modifiers, so the
  // confirm button drops ⌘↩ and Ctrl-↩ itself.
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
    }
  };

  return (
    <fieldset className="border-l-destructive mt-3 mb-0 flex min-w-0 flex-col gap-2 rounded-md border border-l-4 px-3 py-2.5">
      <legend className="float-left p-0 font-semibold">{question(command, count, line)}</legend>
      <p className="m-0 flex flex-wrap items-center gap-2">
        <EffectBadge effect={effectOf(command)} />
        {effectsInWords(command)}
      </p>
      <code className="bg-code w-max max-w-full overflow-x-auto rounded px-1.5 py-0.5 font-mono text-xs">
        {line}
      </code>
      {stale && (
        <p className="text-destructive m-0 text-xs">
          Stale: the composer changed after this preview. Preview again to confirm.
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        <Button ref={cancel} type="button" size="sm" variant="outline" onClick={onDecline}>
          Cancel
        </Button>
        <Button
          type="button"
          size="sm"
          className="bg-destructive text-card hover:bg-destructive/90"
          disabled={stale}
          onKeyDown={onKeyDown}
          onClick={onConfirm}
        >
          {confirmLabel(command, count)}
        </Button>
      </div>
    </fieldset>
  );
}

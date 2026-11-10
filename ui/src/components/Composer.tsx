import { Eye, Play } from "lucide-react";
import { type KeyboardEvent, useEffect, useId, useRef } from "react";
import type { Argument, Command, Tool } from "@/api/model";
import type { Value } from "@/api/runs";
import { useMountTime } from "@/clock";
import { Button } from "@/components/ui/button";
import { dayOf } from "@/lib/cost";
import { chosenOf, type Draft, type FieldGroup, formOf, missing, setValue } from "@/lib/form";
import { chooserLinkOf } from "@/lib/links";
import { needsConfirmation, whyNotRunnable } from "@/lib/rail";
import { ChipsInput } from "./ChipsInput";
import { CostCard } from "./CostCard";
import { IdChooser } from "./IdChooser";
import { CommandPathText } from "./Rail";

/** A refused argument and the words the server gave for it. */
export type FieldError = { name: string; words: string };

type Props = {
  tool: Tool;
  command: Command;
  draft: Draft;
  editing: number | null;
  /**
   * Changes each time the composer should take focus, such as on each Edit,
   * or null when it should not.
   */
  focus: number | null;
  readOnly: boolean;
  fieldError: FieldError | null;
  /** True while the line holds a word the tool would refuse, which keeps Run off. */
  lineRefused: boolean;
  onChange: (draft: Draft) => void;
  onRun: () => void;
  onStopEditing: () => void;
  onShowRun: (number: number) => void;
};

/**
 * The composer's form, under the command line: what the command costs an
 * agent, when the tool keeps figures for it, the command's required
 * fields, then its other options under "More options", and Run, which ⌘↩
 * or Ctrl-↩ presses from anywhere in the form. A command that needs a
 * confirmation has Preview in place of Run, and `onRun` previews it. Under
 * `--read-only` a command that is not read-only shows why in place of the
 * button. It sends structured values.
 */
export function Composer(props: Props) {
  const { command, draft, editing, focus, readOnly, lineRefused, onRun, onStopEditing, onShowRun } =
    props;
  const form = formOf(command);
  const unset = missing(command, draft);
  const blocked = whyNotRunnable(command, readOnly);
  const runnable = blocked === null && unset.length === 0 && !lineRefused;
  const formRef = useRef<HTMLFormElement>(null);
  // The cost card's period ends on the day the form opened.
  const now = useMountTime();

  // Each Edit moves focus to the first field, which may be a text field, a
  // chips field, a list or a checkbox, and selects its text, so the person
  // can type the new value at once. A first field under "More options" opens
  // that section first.
  useEffect(() => {
    if (focus === null) {
      return;
    }
    const field = formRef.current?.querySelector<HTMLInputElement | HTMLSelectElement>(
      "input[type=text], input[type=checkbox], select",
    );
    const section = field?.closest("details");
    if (section !== null && section !== undefined) {
      section.open = true;
    }
    field?.focus();
    if (field instanceof HTMLInputElement && field.type === "text") {
      field.select();
    }
  }, [focus]);
  // Enter in a text field submits the form even when no Run button
  // shows, so both ways in check that the command may run.
  const runIfRunnable = () => {
    if (runnable) {
      onRun();
    }
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      runIfRunnable();
    } else if (event.key === "Escape" && editing !== null) {
      onStopEditing();
    }
  };

  return (
    // biome-ignore lint/a11y/noNoninteractiveElementInteractions: ⌘↩ and Esc reach the form from whichever of its fields has focus
    <form
      ref={formRef}
      aria-label="Composer"
      onKeyDown={onKeyDown}
      onSubmit={(event) => {
        event.preventDefault();
        runIfRunnable();
      }}
      className="flex min-h-0 flex-col"
    >
      {/* The fields scroll on their own, so the Run row below stays in view. */}
      <div className="-mx-1 min-h-0 overflow-y-auto px-1">
        <div className="mb-1 flex flex-wrap items-baseline gap-2.5">
          <h2 className="m-0 text-lg font-semibold">
            <CommandPathText command={command} />
          </h2>
          {editing !== null && (
            <span className="text-muted-foreground text-xs">
              Editing run {editing}{" "}
              <button
                type="button"
                className="text-primary hover:underline"
                onClick={() => onShowRun(editing)}
              >
                Show run {editing}
              </button>
            </span>
          )}
        </div>
        {command.about !== null && (
          <p className="text-muted-foreground mt-0 mb-2">{command.about}</p>
        )}
        {command.cost !== null && (
          <CostCard
            cost={command.cost}
            command={command.path.join(" ")}
            since={props.tool.figures_since}
            today={dayOf(new Date(now))}
          />
        )}
        <div className="flex flex-col gap-2.5">
          {form.required.map((group) => (
            <Field key={group.choices[0]?.name} group={group} {...props} />
          ))}
        </div>
        {form.more.length > 0 && (
          <details className="mt-2.5">
            <summary className="text-primary inline-flex min-h-8 cursor-pointer items-center text-xs hover:underline">
              More options
            </summary>
            <div className="mt-2 flex flex-col gap-2.5">
              {form.more.map((group) => (
                <Field key={group.choices[0]?.name} group={group} {...props} />
              ))}
            </div>
          </details>
        )}
      </div>
      <div className="mt-3 flex shrink-0 flex-wrap items-center gap-3">
        {blocked === null ? (
          <Button type="submit" size="sm" disabled={!runnable}>
            {needsConfirmation(command) ? (
              <>
                <Eye aria-hidden="true" />
                Preview
              </>
            ) : (
              <>
                <Play aria-hidden="true" />
                Run
              </>
            )}
            <kbd className="font-mono text-xs opacity-80">⌘↩</kbd>
          </Button>
        ) : (
          <p className="text-muted-foreground m-0">{blocked}</p>
        )}
      </div>
    </form>
  );
}

type FieldProps = Props & { group: FieldGroup };

/**
 * One field, with a switch between alternatives when it has them, and the
 * IDs chooser when an inferred link fills the chosen argument.
 */
function Field({ tool, command, group, draft, onChange, fieldError }: FieldProps) {
  const chosen = chosenOf(draft, group);
  const [first] = group.choices;
  if (chosen === undefined || first === undefined) {
    return null;
  }
  const link = chooserLinkOf(tool, command, chosen.name);
  const error =
    fieldError !== null && group.choices.some((each) => each.name === fieldError.name)
      ? fieldError.words
      : null;
  return (
    <div className="flex flex-col gap-1">
      {group.choices.length > 1 && (
        <fieldset className="flex min-h-8 w-max gap-0.5 rounded-md border p-1 text-xs">
          <legend className="sr-only">Give {first.name} or another argument</legend>
          {group.choices.map((choice) => (
            <label
              key={choice.name}
              className="has-checked:bg-muted has-focus-visible:outline-ring inline-flex cursor-pointer items-center rounded px-2 font-mono has-checked:font-semibold has-focus-visible:outline-2"
            >
              <input
                type="radio"
                className="sr-only"
                name={`choice-${first.name}`}
                checked={chosen.name === choice.name}
                onChange={() =>
                  onChange({ ...draft, chosen: { ...draft.chosen, [first.name]: choice.name } })
                }
              />
              {choice.name}
            </label>
          ))}
        </fieldset>
      )}
      <Input
        argument={chosen}
        value={draft.values[chosen.name]}
        error={error}
        onValue={(value) => onChange(setValue(draft, chosen.name, value))}
      />
      {link !== null && (
        <IdChooser
          tool={tool}
          link={link}
          many={chosen.arity.kind === "many"}
          value={draft.values[chosen.name]}
          onValue={(value) => onChange(setValue(draft, chosen.name, value))}
        />
      )}
    </div>
  );
}

type InputProps = {
  argument: Argument;
  value: Value | undefined;
  error: string | null;
  onValue: (value: Value | null) => void;
};

/** The input of one argument: a checkbox, a text field, a list or chips. */
function Input({ argument, value, error, onValue }: InputProps) {
  const id = useId();
  const label =
    argument.crumbkit_flag === "--long" ? "Long columns" : (argument.help.line ?? argument.name);
  const errorId = `${id}-error`;
  const name = (
    <span className="text-faint-foreground font-mono text-xs font-normal">{argument.name}</span>
  );
  // Red marks a destructive effect and a failed exit, and nothing else, so
  // the word stays quiet.
  const required = argument.required ? (
    <span className="text-faint-foreground text-xs font-normal">required</span>
  ) : null;
  const message =
    error === null ? null : (
      <span id={errorId} className="text-destructive text-xs">
        {error}
      </span>
    );
  const invalid = error === null ? undefined : true;
  const described = error === null ? undefined : errorId;
  switch (argument.arity.kind) {
    case "flag":
    case "count":
      return (
        <label className="inline-flex items-center gap-2">
          <input
            type="checkbox"
            checked={value === true}
            aria-invalid={invalid}
            aria-describedby={described}
            onChange={(event) => onValue(event.target.checked ? true : null)}
          />
          <span>{label}</span> {name}
          {message}
        </label>
      );
    case "one": {
      const text = typeof value === "string" ? value : "";
      const control =
        argument.possible_values.length > 0 ? (
          <select
            id={id}
            value={text}
            aria-invalid={invalid}
            aria-describedby={described}
            onChange={(event) => onValue(event.target.value === "" ? null : event.target.value)}
            className="border-input bg-background focus-visible:border-ring transition-tint min-h-8 w-max rounded-md border px-2 py-1"
          >
            <option value="">{argument.defaults[0] ?? "None"}</option>
            {argument.possible_values.map((possible) => (
              <option key={possible} value={possible}>
                {possible}
              </option>
            ))}
          </select>
        ) : (
          <input
            id={id}
            type="text"
            value={text}
            placeholder={argument.defaults.join(", ")}
            aria-invalid={invalid}
            aria-describedby={described}
            onChange={(event) => onValue(event.target.value === "" ? null : event.target.value)}
            className="border-input bg-background focus-visible:border-ring transition-tint min-h-8 max-w-sm rounded-md border px-2 py-1 font-mono"
          />
        );
      return (
        <div className="flex flex-col gap-1">
          <label htmlFor={id} className="flex flex-wrap items-baseline gap-2 text-xs font-medium">
            {label} {name} {required}
          </label>
          {control}
          {message}
        </div>
      );
    }
    case "many":
      return (
        <ChipsInput
          label={label}
          name={name}
          required={required}
          items={Array.isArray(value) ? value : []}
          error={message}
          errorId={described}
          onItems={(items) => onValue(items.length === 0 ? null : items)}
        />
      );
  }
}

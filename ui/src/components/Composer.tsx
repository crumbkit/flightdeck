import { Eye, Play } from "lucide-react";
import { type KeyboardEvent, useEffect, useId, useRef } from "react";
import type { Argument, Command } from "@/api/model";
import type { Value } from "@/api/runs";
import { Button } from "@/components/ui/button";
import { chosenOf, type Draft, type FieldGroup, formOf, missing, setValue } from "@/lib/form";
import { needsConfirmation, whyNotRunnable } from "@/lib/rail";
import { ChipsInput } from "./ChipsInput";
import { CommandPathText } from "./Rail";

/** A refused argument and the words the server gave for it. */
export type FieldError = { name: string; words: string };

type Props = {
  command: Command;
  draft: Draft;
  editing: number | null;
  readOnly: boolean;
  fieldError: FieldError | null;
  onChange: (draft: Draft) => void;
  onRun: () => void;
  onStopEditing: () => void;
  onShowRun: (number: number) => void;
};

/**
 * The composer, docked under the transcript: the command's required
 * fields, then its other options under "More options", and Run, which ⌘↩
 * or Ctrl-↩ presses from anywhere in the form. A command that needs a
 * confirmation has Preview in place of Run, and `onRun` previews it. Under
 * `--read-only` a command that is not read-only shows why in place of the
 * button. It sends structured values.
 */
export function Composer(props: Props) {
  const { command, draft, editing, readOnly, onRun, onStopEditing, onShowRun } = props;
  const form = formOf(command);
  const unset = missing(command, draft);
  const blocked = whyNotRunnable(command, readOnly);
  const formRef = useRef<HTMLFormElement>(null);

  // Edit moves focus to the first text field or list and selects its
  // text, so the person can type the new value at once.
  useEffect(() => {
    if (editing === null) {
      return;
    }
    const field = formRef.current?.querySelector<HTMLInputElement | HTMLSelectElement>(
      "input[type=text], select",
    );
    field?.focus();
    if (field instanceof HTMLInputElement) {
      field.select();
    }
  }, [editing]);
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      if (blocked === null && unset.length === 0) {
        onRun();
      }
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
        onRun();
      }}
      className="bg-card max-h-1/2 shrink-0 overflow-y-auto border-t px-4 py-3"
    >
      <div className="mb-1 flex flex-wrap items-baseline gap-2.5">
        <h2 className="m-0 text-lg font-semibold">
          <CommandPathText command={command} />
        </h2>
        {editing !== null && (
          <span className="text-muted-foreground text-xs">
            Editing run {editing}{" "}
            <button type="button" className="text-primary" onClick={() => onShowRun(editing)}>
              Show run {editing}
            </button>
          </span>
        )}
      </div>
      {command.about !== null && <p className="text-muted-foreground mt-0 mb-2">{command.about}</p>}
      <div className="flex flex-col gap-2.5">
        {form.required.map((group) => (
          <Field key={group.choices[0]?.name} group={group} {...props} />
        ))}
      </div>
      {form.more.length > 0 && (
        <details className="mt-2.5">
          <summary className="text-primary cursor-pointer text-xs">More options</summary>
          <div className="mt-2 flex flex-col gap-2.5">
            {form.more.map((group) => (
              <Field key={group.choices[0]?.name} group={group} {...props} />
            ))}
          </div>
        </details>
      )}
      <div className="mt-3 flex flex-wrap items-center gap-3">
        {blocked === null ? (
          <Button type="submit" size="sm" disabled={unset.length > 0}>
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

/** One field, with a switch between alternatives when it has them. */
function Field({ group, draft, onChange, fieldError }: FieldProps) {
  const chosen = chosenOf(draft, group);
  const [first] = group.choices;
  if (chosen === undefined || first === undefined) {
    return null;
  }
  const error =
    fieldError !== null && group.choices.some((each) => each.name === fieldError.name)
      ? fieldError.words
      : null;
  return (
    <div className="flex flex-col gap-1">
      {group.choices.length > 1 && (
        <fieldset className="flex w-max gap-0.5 rounded-md border p-0.5 text-xs">
          <legend className="sr-only">Give {first.name} or another argument</legend>
          {group.choices.map((choice) => (
            <label
              key={choice.name}
              className="has-checked:bg-muted has-focus-visible:outline-ring cursor-pointer rounded px-2 py-0.5 font-mono has-checked:font-semibold has-focus-visible:outline-2"
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
  const label = argument.help.line ?? argument.name;
  const errorId = `${id}-error`;
  const name = (
    <span className="text-faint-foreground font-mono text-xs font-normal">{argument.name}</span>
  );
  const required = argument.required ? (
    <span className="text-destructive text-xs font-medium">required</span>
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
            className="border-input bg-card w-max rounded-md border px-2 py-1"
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
            className="border-input bg-card max-w-sm rounded-md border px-2 py-1 font-mono"
          />
        );
      return (
        <div className="flex flex-col gap-1">
          <label htmlFor={id} className="flex flex-wrap items-baseline gap-2 text-xs font-semibold">
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

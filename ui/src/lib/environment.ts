import type { Variable } from "@/api/model";
import { counted } from "@/lib/words";

/** What stops the tool about a variable: it is required and unset, or its value is invalid. */
export type Wrong = "unset" | "invalid" | null;

/**
 * Returns true when the variable is required and has no value. Its `unset`
 * text starts with the rule of local mode, such as "required" or
 * "required; production optional", and only a variable without a default
 * has the state "unset".
 */
function requiredUnset(variable: Variable): boolean {
  return variable.value.state === "unset" && /^required\b/.test(variable.unset);
}

/** Returns what stops the tool about the variable, or null when nothing does. */
export function wrongOf(variable: Variable): Wrong {
  if (variable.value.state === "invalid") {
    return "invalid";
  }
  return requiredUnset(variable) ? "unset" : null;
}

/**
 * Returns the sentence under a row that stops the tool: what the tool does
 * without a required value, or the check that an invalid value fails. A
 * declaration that states neither gets the page's own sentence.
 */
export function wrongWords(variable: Variable): string | null {
  switch (wrongOf(variable)) {
    case "unset":
      return variable.effect ?? "The tool needs a value for this variable.";
    case "invalid": {
      if (variable.check !== null) {
        return variable.check;
      }
      const { text } = variable.value;
      const subject = text === null ? "The value" : text;
      return `${subject} does not fit the type ${variable.type}.`;
    }
    case null:
      return null;
  }
}

/**
 * Returns the view's summary, such as "16 declared · 1 required unset ·
 * 1 invalid". The count includes the variables that crumbkit reads.
 */
export function environmentSummary(variables: Variable[]): string {
  const unset = variables.filter((each) => wrongOf(each) === "unset").length;
  const invalid = variables.filter((each) => wrongOf(each) === "invalid").length;
  return [
    `${variables.length.toLocaleString("en")} declared`,
    unset > 0 ? `${unset.toLocaleString("en")} required unset` : null,
    invalid > 0 ? `${invalid.toLocaleString("en")} invalid` : null,
  ]
    .filter((each) => each !== null)
    .join(" · ");
}

/** Returns the view's footer count, such as "16 variables". */
export function variablesCount(variables: Variable[]): string {
  return counted(variables.length, "variable");
}

/**
 * Returns true when the run has a value that the page may not show: the
 * tool withholds it, so it has no text, though the variable is set.
 */
export function isWithheld(variable: Variable): boolean {
  const { state, text } = variable.value;
  return text === null && (state === "set" || state === "invalid");
}

/**
 * Returns the default of an `unset` text such as "default 8080", for the
 * page to set in code type, or null for any other rule, such as
 * "optional" or one that differs in production.
 */
export function defaultOf(unset: string): string | null {
  const match = /^default (.+)$/.exec(unset);
  return match === null || match[1] === undefined || match[1].includes(";") ? null : match[1];
}

import type { Tool } from "@/api/model";
import type { RunRequest } from "@/api/runs";
import { isPlaceholder, requestOfWords, words } from "./example";

/**
 * Returns the run request of a call that a failure's recovery or a
 * result's next step names, such as `shelf books show <ID>...`, or null
 * when the call names no command of the tool. A placeholder leaves its
 * field empty. With `values`, the values of the failure field that a
 * recovery's `all` names, the first placeholder takes every value in its
 * place.
 */
export function requestOfCall(
  tool: Tool,
  call: string,
  values: readonly unknown[] | null,
): RunRequest | null {
  const split = words(call);
  const at = split.findIndex(isPlaceholder);
  if (values !== null && at !== -1) {
    split.splice(at, 1, ...values.map(String));
  }
  return requestOfWords(tool, split);
}

/**
 * Returns a call's words without the tool's name, as a chip shows it, such
 * as "books show <ID>…".
 */
export function callLabel(tool: Tool, call: string): string {
  const split = words(call);
  const rest = split[0] === tool.name ? split.slice(1) : split;
  return rest.join(" ").replaceAll("...", "…");
}

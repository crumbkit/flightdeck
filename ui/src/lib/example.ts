import type { Tool } from "@/api/model";
import type { RunRequest } from "@/api/runs";
import { draftFrom, requestOf } from "./form";
import { parseLine, stripPrompt } from "./line";

/**
 * Returns the run request of an example's command line, such as
 * `shelf books show 2 5`, or null when the line does not start with the
 * tool's name and one of its commands, or the tool would refuse it.
 * crumbkit's flags that flightdeck sets itself are dropped, and so are
 * `--fields` with its value and `--long`, which only choose columns.
 */
export function requestOfExample(tool: Tool, line: string): RunRequest | null {
  const unprompted = stripPrompt(tool, line);
  if (unprompted === line) {
    return null;
  }
  const { command, values, problem } = parseLine(tool, unprompted, null);
  if (command === null || problem !== null) {
    return null;
  }
  return requestOf(command, draftFrom(command, { path: command.path, values }));
}

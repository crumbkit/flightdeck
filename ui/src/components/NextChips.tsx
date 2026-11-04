import { useState } from "react";
import type { Tool } from "@/api/model";
import type { Step } from "@/api/result";
import type { RunRequest } from "@/api/runs";
import { callLabel, requestOfCall } from "@/lib/call";
import { Chip } from "./Chip";

/** How many chips show before "+N more". */
const shownAtFirst = 3;

type Props = { steps: Step[]; tool: Tool; onCompose: (request: RunRequest) => void };

/**
 * The next steps that a result proposes, after its footer: the first as
 * the primary chip, the next two as quiet chips, and the rest behind
 * "+N more". Each chip fills the composer with the step's call and runs
 * nothing.
 *
 * A chip carries a step whose commands are all calls of this tool, and
 * opens the first call. A step that also runs another program, such as
 * `rm`, is written out as its sentence and its command lines, because the
 * page runs only the tool.
 */
export function NextChips({ steps, tool, onCompose }: Props) {
  const [all, setAll] = useState(false);
  const chips = steps.flatMap((step) => {
    const calls = step.commands.flatMap((each) => ("call" in each ? [each.call] : []));
    const [call] = calls;
    const request = call === undefined ? null : requestOfCall(tool, call, null);
    return call !== undefined && request !== null && calls.length === step.commands.length
      ? [{ step, call, request }]
      : [];
  });
  const written = steps.filter((step) => !chips.some((chip) => chip.step === step));
  if (steps.length === 0) {
    return null;
  }
  const shown = all ? chips : chips.slice(0, shownAtFirst);
  const more = chips.length - shown.length;
  return (
    <div className="mt-2 flex flex-col gap-1.5 text-xs">
      {chips.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="text-muted-foreground">Next:</span>
          {shown.map(({ step, call, request }, index) => (
            <Chip
              key={call}
              primary={index === 0}
              title={`${call} ${step.text}`}
              onClick={() => onCompose(request)}
            >
              {callLabel(tool, call)}
            </Chip>
          ))}
          {more > 0 && (
            <button type="button" className="text-primary" onClick={() => setAll(true)}>
              +{more} more
            </button>
          )}
        </div>
      )}
      {written.map((step) => (
        <p key={step.text} className="text-muted-foreground m-0">
          {step.text}{" "}
          {step.commands.map((each) => (
            <code key={"call" in each ? each.call : each.run} className="mr-2 font-mono">
              {"call" in each ? each.call : each.run}
            </code>
          ))}
        </p>
      ))}
    </div>
  );
}

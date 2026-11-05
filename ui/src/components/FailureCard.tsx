import type { FailureDocument, Label, Recovery } from "@/api/client";
import type { Tool } from "@/api/model";
import type { RunRequest } from "@/api/runs";
import { callLabel, requestOfCall } from "@/lib/call";
import { Chip } from "./Chip";
import { CopyButton } from "./CopyButton";

type Props = {
  /** The failure document, or null for a failure that came without one. */
  failure: FailureDocument | null;
  /** The card's first line: the failure's message, or the page's words for it. */
  words: string;
  /** The key of a batch's failed item, such as "7 (index 1)", or null. */
  heading: string | null;
  tool: Tool;
  onCompose: (request: RunRequest) => void;
};

/**
 * One failure: its message, its cause, its code with a copy button, its
 * fields, the places in a source it points at, and what to do next. A
 * `run` recovery is the primary chip and each `run` alternative a quiet
 * one, and each chip fills the composer with its call and runs nothing.
 */
export function FailureCard({ failure, words, heading, tool, onCompose }: Props) {
  const cause = failure?.cause;
  const fields = Object.entries(failure?.fields ?? {});
  const repairs = [failure?.recovery, ...(failure?.alternatives ?? [])].flatMap((each) =>
    each === undefined ? [] : [each],
  );
  return (
    <div role="alert" className="border-l-destructive flex flex-col gap-1.5 border-l-4 py-1 pl-3">
      <p className="m-0">
        {heading !== null && <span className="mr-2 font-mono font-semibold">{heading}</span>}
        <span className="font-semibold">{words}</span>
      </p>
      {cause !== undefined && cause !== words && (
        <p className="text-muted-foreground m-0">{cause}</p>
      )}
      {failure !== null && (
        <span className="flex items-center gap-2">
          <code className="bg-code text-muted-foreground rounded px-1.5 font-mono text-xs">
            {failure.code}
          </code>
          <CopyButton text={failure.code} label="Copy code" />
        </span>
      )}
      {fields.length > 0 && (
        <dl className="m-0 flex flex-col gap-0.5 text-xs">
          {fields.map(([name, value]) => (
            <div key={name} className="flex gap-3">
              <dt className="text-faint-foreground w-32 shrink-0 font-mono">{name}</dt>
              <dd className="m-0 min-w-0 font-mono">{JSON.stringify(value)}</dd>
            </div>
          ))}
        </dl>
      )}
      {failure?.labels?.map((label) => (
        <LabelView key={`${label.offset}-${label.length}`} label={label} />
      ))}
      {repairs.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5">
          {repairs.map((repair, index) => (
            <Repair
              // A failure's repairs never change order, and two may be alike.
              // biome-ignore lint/suspicious/noArrayIndexKey: a failure's repairs are fixed
              key={index}
              repair={repair}
              primary={index === 0 && repair === failure?.recovery}
              fields={failure?.fields ?? {}}
              tool={tool}
              onCompose={onCompose}
            />
          ))}
        </div>
      )}
    </div>
  );
}

type RepairProps = {
  repair: Recovery;
  primary: boolean;
  fields: Record<string, unknown>;
  tool: Tool;
  onCompose: (request: RunRequest) => void;
};

/** One recovery or alternative: a chip for a call, and words for the rest. */
function Repair({ repair, primary, fields, tool, onCompose }: RepairProps) {
  switch (repair.kind) {
    case "run": {
      const values = repair.all === undefined ? null : fields[repair.all];
      const request = requestOfCall(tool, repair.call, Array.isArray(values) ? values : null);
      if (request === null) {
        return <code className="font-mono text-xs">{repair.call}</code>;
      }
      return (
        <Chip primary={primary} title={repair.call} onClick={() => onCompose(request)}>
          {callLabel(tool, repair.call)}
        </Chip>
      );
    }
    case "retry":
      return (
        <span className="text-muted-foreground text-xs">Run the same command again later.</span>
      );
    case "hint":
      return <span className="text-muted-foreground text-xs">{repair.text}</span>;
    case "none":
      return <span className="text-muted-foreground text-xs">{repair.reason}</span>;
  }
}

/**
 * A place in a source that a failure points at, with its lines and the
 * span underlined.
 *
 * The span's length counts bytes and the page underlines that many
 * characters from the column, which is exact for ASCII text. A span over
 * other characters is underlined a little long or short, and counting the
 * line's bytes replaces this when a tool's sources need it.
 */
function LabelView({ label }: { label: Label }) {
  const place = [label.name, label.line, label.column].filter((each) => each !== undefined);
  const { excerpt, line, column } = label;
  return (
    <figure className="m-0 text-xs">
      <figcaption className="text-muted-foreground">
        {place.length > 0 && <span className="font-mono">{place.join(":")} </span>}
        {label.text}
      </figcaption>
      {excerpt !== undefined && line !== undefined && column !== undefined && (
        <pre className="bg-code m-0 mt-1 overflow-x-auto rounded p-2 font-mono">
          {excerpt.text.split("\n").map((text, index) => {
            const number = excerpt.line + index;
            if (number !== line) {
              return (
                <span key={number} className="block">
                  {text}
                </span>
              );
            }
            const start = column - 1;
            const end = start + Math.max(label.length, 1);
            return (
              <span key={number} className="block">
                {text.slice(0, start)}
                <span className="decoration-destructive underline decoration-wavy">
                  {text.slice(start, end)}
                </span>
                {text.slice(end)}
              </span>
            );
          })}
        </pre>
      )}
    </figure>
  );
}

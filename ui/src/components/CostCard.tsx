import { cn } from "cn";
import { type ReactNode, useId } from "react";
import type { Cost, Day, Figures } from "@/api/model";
import {
  bytesWords,
  grouped,
  type Phrased,
  type PhrasedKind,
  percentOf,
  periodWords,
  savedCalls,
  stripFindings,
  tokensWords,
} from "@/lib/cost";
import { elapsedWords } from "@/lib/duration";
import { counted } from "@/lib/words";

type Props = {
  cost: Cost;
  /** The command's words below the tool's name, such as "books show". */
  command: string;
  /** The day the tool's figures start, or null. */
  since: Day | null;
  today: Day;
};

/**
 * What a command costs the agents that call it, from the tool's own
 * `self figures list` and `self usage`: its wall time, the output an agent
 * reads, its failures, the calls an agent could have saved and the tool's
 * findings. Each figure shows only when the tool reports it.
 */
export function CostCard({ cost, command, since, today }: Props) {
  const titleId = useId();
  const { figures, findings } = cost;
  const saved = savedCalls(cost);
  // Each kind shows once, so the kind keys its item.
  const phrased = stripFindings(findings).flatMap((finding) => {
    const sentence = sentenceOf(finding, figures, command);
    return sentence === null ? [] : [{ finding, sentence }];
  });
  const tiles = [
    figures.p95_ms !== null && (
      <Tile key="time" title="Wall time">
        <Pair>
          <Figure value={elapsedWords(figures.p95_ms)} label="95th percentile" />
        </Pair>
      </Tile>
    ),
    figures.p95_out_bytes !== null && (
      <Tile key="output" title="Output an agent reads">
        <Pair>
          <Figure
            value={bytesWords(figures.p95_out_bytes)}
            label={`95th, ${tokensWords(figures.p95_out_bytes)}`}
          />
        </Pair>
      </Tile>
    ),
    figures.failed !== null && (
      <Tile key="failed" title="Failed">
        <Failed failed={figures.failed} runs={figures.runs} />
      </Tile>
    ),
    saved.length > 0 && (
      <Tile key="saved" title="Calls an agent could have saved">
        <Rows>
          {saved.map((each) => (
            <Row key={each.label} count={each.count}>
              {each.label}
            </Row>
          ))}
        </Rows>
      </Tile>
    ),
  ].filter(Boolean);

  return (
    <section aria-labelledby={titleId} className="bg-background mb-2.5 rounded-md border">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 border-b px-3 py-2">
        <h3 id={titleId} className="m-0 font-semibold">
          Cost to an agent
        </h3>
        <span className="text-muted-foreground text-xs">
          {periodWords(figures.runs, since, today)}
        </span>
      </div>
      {tiles.length > 0 && (
        // Each tile draws its right and bottom lines, and the wrapper clips
        // the ones that fall on the card's edge, so the dividers are right
        // however the tiles wrap.
        <div className="overflow-hidden">
          <div className="grid-cols-tiles -mr-px -mb-px grid">{tiles}</div>
        </div>
      )}
      {phrased.length > 0 && (
        <ul
          aria-label="Findings"
          className="m-0 flex list-none flex-wrap gap-x-4 gap-y-1.5 border-t px-3 py-2 text-xs"
        >
          {phrased.map(({ finding, sentence }) => (
            <li key={finding.kind} className="inline-flex flex-wrap items-center gap-1.5">
              <Badge kind={finding.kind} />
              <span>{sentence}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function Tile({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5 border-r border-b px-3 py-2.5">
      <h4 className="text-muted-foreground m-0 text-xs font-medium">{title}</h4>
      {children}
    </div>
  );
}

/**
 * The figures of one tile side by side. Today it holds the 95th
 * percentile alone; a tool that reports the median adds it here.
 */
function Pair({ children }: { children: ReactNode }) {
  return <div className="flex items-baseline gap-4">{children}</div>;
}

function Figure({ value, label, bad = false }: { value: string; label: string; bad?: boolean }) {
  return (
    <div>
      <div
        className={cn(
          "text-xl leading-tight font-semibold tabular-nums",
          bad && "text-destructive",
        )}
      >
        {value}
      </div>
      <div className="text-faint-foreground text-2xs">{label}</div>
    </div>
  );
}

function Rows({ children }: { children: ReactNode }) {
  return (
    <dl className="grid-cols-counts m-0 grid gap-x-3 gap-y-0.5 text-xs tabular-nums">{children}</dl>
  );
}

function Row({ count, children }: { count: number; children: ReactNode }) {
  return (
    <>
      <dt className="inline-flex min-w-0 items-center gap-1.5">{children}</dt>
      <dd className="m-0 text-right font-mono">{grouped(count)}</dd>
    </>
  );
}

/**
 * The failed runs of all runs. A tool that reports failures by code adds
 * a bar and a row for each code here.
 */
function Failed({ failed, runs }: { failed: number; runs: number }) {
  const percent = percentOf(failed, runs);
  return (
    <Pair>
      <Figure
        value={`${grouped(failed)} of ${grouped(runs)}`}
        label={percent === null ? "of runs" : `${percent}% of runs`}
        bad={failed > 0}
      />
    </Pair>
  );
}

/** The short word of each kind's badge, and whether it is red. */
const badges: Record<PhrasedKind, { label: string; red: boolean }> = {
  slow_command: { label: "slow", red: true },
  help_loop: { label: "help loop", red: false },
  retry_after_failure: { label: "retry", red: false },
  heavy_output: { label: "heavy output", red: false },
};

function Badge({ kind }: { kind: PhrasedKind }) {
  const badge = badges[kind];
  return (
    <span
      className={cn(
        "text-2xs inline-flex items-center rounded-sm px-1.5 leading-4.5 font-medium",
        badge.red ? "bg-destructive-soft text-destructive" : "bg-additive-soft text-additive",
      )}
    >
      {badge.label}
    </span>
  );
}

/**
 * Returns a finding in the card's words, with codes and the command in
 * mono, or null when the tool left out a figure the sentence needs.
 */
function sentenceOf(finding: Phrased, figures: Figures, command: string): ReactNode | null {
  const name = <code>{command}</code>;
  switch (finding.kind) {
    case "slow_command": {
      const ms = finding.p95_ms ?? figures.p95_ms;
      return ms === null ? null : `The 95th percentile wall time is ${elapsedWords(ms)}.`;
    }
    case "help_loop":
      // crumbkit-usage counts a help loop's pages, not its sessions.
      return (
        <>
          Agents read 3 or more help pages before {name} first succeeded,{" "}
          {counted(finding.runs, "page")} in all.
        </>
      );
    case "retry_after_failure":
      return (
        <>
          After {finding.code === null ? "a failure" : <code>{finding.code}</code>}, agents ran{" "}
          {name} again {counted(finding.runs, "time")} within 60 s.
        </>
      );
    case "heavy_output": {
      const bytes = finding.p95_out_bytes ?? figures.p95_out_bytes;
      return bytes === null
        ? null
        : `The 95th percentile output is ${bytesWords(bytes)}, ${tokensWords(bytes)}.`;
    }
  }
}

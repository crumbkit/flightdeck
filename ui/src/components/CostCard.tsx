import { cn } from "cn";
import { type ReactNode, useId } from "react";
import type { Cost, Day, Figures, Run } from "@/api/model";
import {
  bytesWords,
  type CodeRow,
  commandHistory,
  grouped,
  type Phrased,
  type PhrasedKind,
  percentOf,
  periodWords,
  savedCalls,
  stripFindings,
  tokensWords,
  topCodes,
  type UsageTab,
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
  /** Each run in the tool's usage log, or null when the tool lists none. */
  history?: Run[] | null;
  /** Opens a usage view on this command, or null when there is none to open. */
  onUsage?: ((tab: UsageTab) => void) | null;
};

/** The usage views that the card links to, in the order it shows them. */
const tabs: [UsageTab, string][] = [
  ["steps", "Steps"],
  ["sessions", "Sessions"],
  ["flags", "Flags"],
];

/**
 * What a command costs the agents that call it, from the tool's own
 * `self figures list`, `self usage` and `self runs list`: its wall time,
 * the output an agent reads, its failures by code, the calls an agent
 * could have saved and the tool's findings. Each figure shows only when
 * the tool reports it.
 */
export function CostCard({ cost, command, since, today, history = null, onUsage = null }: Props) {
  const titleId = useId();
  const { figures, findings } = cost;
  const saved = savedCalls(cost);
  const ran = history === null ? null : commandHistory(history, command);
  // Each kind shows once, so the kind keys its item.
  const phrased = stripFindings(findings).flatMap((finding) => {
    const sentence = sentenceOf(finding, figures, command);
    return sentence === null ? [] : [{ finding, sentence }];
  });
  const tiles = [
    figures.p95_ms !== null && (
      <Tile key="time" title="Wall time">
        <Pair>
          {figures.p50_ms !== null && (
            <Figure value={elapsedWords(figures.p50_ms)} label="median" />
          )}
          <Figure value={elapsedWords(figures.p95_ms)} label="95th percentile" />
        </Pair>
        <Track median={figures.p50_ms} high={figures.p95_ms} label={(each) => elapsedWords(each)} />
      </Tile>
    ),
    figures.p95_out_bytes !== null && (
      <Tile key="output" title="Output an agent reads">
        <Pair>
          {figures.p50_out_bytes !== null && (
            <Figure
              value={bytesWords(figures.p50_out_bytes)}
              label={`median, ${tokensWords(figures.p50_out_bytes)}`}
            />
          )}
          <Figure
            value={bytesWords(figures.p95_out_bytes)}
            label={`95th, ${tokensWords(figures.p95_out_bytes)}`}
          />
        </Pair>
        <Track median={figures.p50_out_bytes} high={figures.p95_out_bytes} label={bytesWords} />
      </Tile>
    ),
    figures.failed !== null && (
      <Tile key="failed" title="Failed">
        <Failed
          failed={figures.failed}
          runs={figures.runs}
          codes={ran === null ? [] : topCodes(ran.codes)}
        />
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
          {periodWords(figures.runs, since, today, ran?.sessions ?? null)}
        </span>
        {onUsage !== null && (
          <span className="ml-auto flex items-center gap-1">
            {tabs.map(([tab, label]) => (
              <button
                key={tab}
                type="button"
                onClick={() => onUsage(tab)}
                className="text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-tint inline-flex h-8 items-center rounded-md px-2.5 text-xs font-medium"
              >
                {label}
              </button>
            ))}
          </span>
        )}
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

/** The figures of one tile side by side: the median, when reported, and the 95th percentile. */
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

/**
 * A bar as long as the 95th percentile with the median drawn over it, so
 * the gap between them shows how far the slow runs reach. It shows only
 * when the tool reports the median.
 */
function Track({
  median,
  high,
  label,
}: {
  median: number | null;
  high: number;
  label: (value: number) => string;
}) {
  if (median === null || high === 0) {
    return null;
  }
  return (
    <div
      role="img"
      aria-label={`Median ${label(median)}, 95th percentile ${label(high)}`}
      className="bg-muted relative h-2 rounded-xs"
    >
      <div className="bg-k-ok absolute inset-y-0 left-0 w-full rounded-xs" />
      <div
        className="bg-primary absolute inset-y-0 left-0 rounded-xs"
        style={{ width: `${Math.min(100, (median / high) * 100)}%` }}
      />
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
 * The colour of each failure code's dot and bar, by its place: the first
 * three codes, then the rest together. The colours tell the codes apart
 * within the tile; each sits beside its code's name.
 */
const codeColours = ["bg-k-fail", "bg-k-usage", "bg-k-repeat", "bg-faint-foreground"];

/** The failed runs of all runs, with a bar and a row for each code when the tool lists its runs. */
function Failed({ failed, runs, codes }: { failed: number; runs: number; codes: CodeRow[] }) {
  const percent = percentOf(failed, runs);
  return (
    <>
      <Pair>
        <Figure
          value={`${grouped(failed)} of ${grouped(runs)}`}
          label={percent === null ? "of runs" : `${percent}% of runs`}
          bad={failed > 0}
        />
      </Pair>
      {codes.length > 0 && runs > 0 && (
        <>
          <div
            role="img"
            aria-label="Failures by code"
            className="bg-muted flex h-2 overflow-hidden rounded-xs"
          >
            {codes.map((row, index) => (
              <div
                key={row.code ?? "others"}
                className={codeColours[index]}
                style={{ width: `${(row.count / runs) * 100}%` }}
              />
            ))}
          </div>
          <Rows>
            {codes.map((row, index) => (
              <Row key={row.code ?? "others"} count={row.count}>
                <span
                  aria-hidden="true"
                  className={cn("size-2 shrink-0 rounded-full", codeColours[index])}
                />
                {row.code === null ? (
                  counted(row.others, "other code")
                ) : (
                  <code className="truncate">{row.code}</code>
                )}
              </Row>
            ))}
          </Rows>
        </>
      )}
    </>
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

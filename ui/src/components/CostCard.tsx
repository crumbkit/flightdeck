import { cn } from "cn";
import { type ReactNode, useId } from "react";
import type { Cost, Finding } from "@/api/model";
import {
  badgeOf,
  bytesWords,
  grouped,
  percentOf,
  type RetriedCode,
  retriedCodes,
  runsWords,
  savedCalls,
  tokensWords,
} from "@/lib/cost";
import { elapsedWords } from "@/lib/duration";

/** The dot of each of the first three retried codes; the rest share one. */
const codeDots = ["bg-k-fail", "bg-k-usage", "bg-k-repeat"];

type Props = {
  cost: Cost;
  /** The day the tool's figures start, such as "2026-11-03", or null. */
  since: string | null;
};

/**
 * What a command costs the agents that call it, from the tool's own
 * `self figures list` and `self usage`: its wall time, the output an agent
 * reads, its failures, the calls an agent could have saved and the tool's
 * findings. Each figure shows only when the tool reports it.
 */
export function CostCard({ cost, since }: Props) {
  const titleId = useId();
  const { figures, findings } = cost;
  const saved = savedCalls(cost);
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
        <Failed failed={figures.failed} runs={figures.runs} codes={retriedCodes(findings)} />
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
        <span className="text-muted-foreground text-xs">{runsWords(figures.runs, since)}</span>
      </div>
      {tiles.length > 0 && <div className="grid-cols-tiles grid">{tiles}</div>}
      {findings !== null && findings.length > 0 && (
        <ul
          aria-label="Findings"
          className="m-0 flex list-none flex-wrap gap-x-4 gap-y-1.5 border-t px-3 py-2 text-xs"
        >
          {findings.map((finding, index) => (
            // The tool may print one finding twice, so its place keys it.
            // biome-ignore lint/suspicious/noArrayIndexKey: the list never reorders
            <FindingItem key={index} finding={finding} />
          ))}
        </ul>
      )}
    </section>
  );
}

function Tile({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5 border-r px-3 py-2.5 last:border-r-0">
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
 * The failed runs of all runs, and the codes of the failures that agents
 * ran the command again after, each with how often they did.
 */
function Failed({ failed, runs, codes }: { failed: number; runs: number; codes: RetriedCode[] }) {
  const percent = percentOf(failed, runs);
  const shown = codes.slice(0, codeDots.length);
  const others = codes.slice(codeDots.length);
  return (
    <>
      <Pair>
        <Figure
          value={`${grouped(failed)} of ${grouped(runs)}`}
          label={percent === null ? "of runs" : `${percent}% of runs`}
          bad={failed > 0}
        />
      </Pair>
      {codes.length > 0 && (
        <>
          <div className="text-faint-foreground text-2xs">Ran again after</div>
          <Rows>
            {shown.map((each, index) => (
              <Row key={each.code} count={each.runs}>
                <span
                  aria-hidden="true"
                  className={cn("size-2 shrink-0 rounded-full", codeDots[index])}
                />
                <code className="truncate">{each.code}</code>
              </Row>
            ))}
            {others.length > 0 && (
              <Row count={others.reduce((sum, each) => sum + each.runs, 0)}>
                <span
                  aria-hidden="true"
                  className="bg-faint-foreground size-2 shrink-0 rounded-full"
                />
                {others.length} other {others.length === 1 ? "code" : "codes"}
              </Row>
            )}
          </Rows>
        </>
      )}
    </>
  );
}

function FindingItem({ finding }: { finding: Finding }) {
  const badge = badgeOf(finding);
  return (
    <li className="inline-flex items-center gap-1.5">
      <span
        className={cn(
          "text-2xs inline-flex items-center rounded-sm px-1.5 leading-4.5 font-medium",
          badge.tone === "slow"
            ? "bg-destructive-soft text-destructive"
            : "bg-additive-soft text-additive",
        )}
      >
        {badge.label}
      </span>
      <span>
        {finding.suggestion} ({grouped(finding.runs)} {finding.runs === 1 ? "run" : "runs"})
      </span>
    </li>
  );
}

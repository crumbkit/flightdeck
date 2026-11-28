import { useQuery } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { ApiError } from "@/api/client";
import { describeFailure, isSignedOut } from "@/api/failure";
import {
  type Freshness,
  type Graph as GraphRecord,
  type GraphRow,
  type WorkflowShow,
  workflowQuery,
  workflowsQuery,
} from "@/api/workflows";
import { Failure } from "@/components/Failure";
import { go, hrefOf } from "@/lib/place";
import { layout } from "@/lib/workflowLayout";
import "./workflows.css";
import { boardGet } from "@/lib/shell";
import { Graph } from "./Graph";
import {
  againstRunning,
  headline,
  phoneLabels,
  type Selection,
  selectionOf,
  stillThere,
} from "./model";
import { Panel } from "./Panel";
import { usePhone } from "./phone";
import { RunSheet } from "./RunSheet";
import { countWord, Glyph, lastSegment, short } from "./words";

/** The width of `text` in `font`, from a canvas, or 7 px a character without one. */
function measurer(): (text: string, font: string) => number {
  const context =
    typeof document === "undefined" ? null : document.createElement("canvas").getContext?.("2d");
  if (context === null || context === undefined) {
    return (text) => text.length * 7;
  }
  return (text, font) => {
    context.font = font;
    return context.measureText(text).width;
  };
}

/** The freshness chip of the source row. */
function FreshnessChip({ freshness, older }: { freshness: Freshness; older: boolean }) {
  if (older) {
    return <span className="wf-chip wf-chip-violet">Older than the running flow</span>;
  }
  switch (freshness.state) {
    case "current":
      return <span className="wf-chip wf-chip-green">Current</span>;
    case "changed":
      return <span className="wf-chip wf-chip-amber">Changed since this reading</span>;
    case "gone":
      return <span className="wf-chip wf-chip-red">Source gone</span>;
    default:
      return (
        <span className="wf-chip" title={freshness.reason}>
          Not checked against the running flow
        </span>
      );
  }
}

/** Returns the day and month of an RFC 3339 time, such as "22 November". */
function dayOf(made: string): string {
  const date = new Date(made);
  if (Number.isNaN(date.getTime())) {
    return made;
  }
  return `${date.getUTCDate()} ${date.toLocaleString("en-GB", { month: "long", timeZone: "UTC" })}`;
}

/** What the page says in place of copying a path that holds a control character. */
const uncopyableText =
  "This record's path holds a control character, so flightdeck does not copy it.";

/** One segment per graph, or a select menu past six graphs. */
function Switch({ rows, slug, phone }: { rows: GraphRow[]; slug: string; phone: boolean }) {
  const choose = (next: string) => go({ view: "workflows", slug: next, selected: null });
  const titles = rows.map((row) => row.title ?? row.slug);
  const labels = phone ? phoneLabels(titles) : titles;
  const name = (row: GraphRow) => labels[rows.indexOf(row)] ?? row.slug;
  if (rows.length > 6) {
    return (
      <select
        className="wf-select"
        aria-label="Workflow"
        value={slug}
        onChange={(event) => choose(event.target.value)}
      >
        {rows.map((row) => (
          <option key={row.slug} value={row.slug}>
            {name(row)}
          </option>
        ))}
      </select>
    );
  }
  return (
    // biome-ignore lint/a11y/useSemanticElements: the mockup's segments are a group of toggle buttons
    <div className="wf-seg" role="group" aria-label="Workflow">
      {rows.map((row) => (
        <button
          key={row.slug}
          type="button"
          aria-pressed={row.slug === slug}
          onClick={() => choose(row.slug)}
        >
          {name(row)}
        </button>
      ))}
    </div>
  );
}

/** The toggle that hides the conditional checks. */
function HideToggle({ on, onChange }: { on: boolean; onChange: (on: boolean) => void }) {
  return (
    <label className="wf-toggle">
      <input type="checkbox" checked={on} onChange={(event) => onChange(event.target.checked)} />
      Hide checks that apply only sometimes
    </label>
  );
}

/** The legend over the swimlane. */
function Legend({ live }: { live: boolean }) {
  const line = (kind: string) => (
    <svg width="22" height="8" aria-hidden="true">
      <path className={`wf-e wf-e-${kind}`} d="M0 4H22" />
    </svg>
  );
  return (
    <div className="wf-legend">
      <span>
        <Glyph kind="step" />
        Step
      </span>
      <span>
        <Glyph kind="gate" />
        Check
      </span>
      <span className="wf-legend-owner">
        <Glyph kind="wait" />
        Waits on you
      </span>
      <span>
        <Glyph kind="choice" />
        Choice
      </span>
      <span>{line("fail")}fails</span>
      <span>{line("retry")}back to fix</span>
      <span>{line("loop")}loops</span>
      <span>
        <span className="wf-legend-sometimes" />
        only sometimes
      </span>
      {live && (
        <>
          <span>
            <i className="wf-dot" />
            open change
          </span>
          <span>
            <i className="wf-dot wf-dot-old" />
            on an older flow
          </span>
        </>
      )}
    </div>
  );
}

/** Says why the graph is not drawn: Board's code, or the check's problems. */
function NotDrawn({ shown }: { shown: WorkflowShow }) {
  if (shown.status === "degraded") {
    return (
      <div className="wf-card" role="alert">
        <p className="m-0 font-medium">Board marks this graph degraded, so it is not drawn.</p>
        <p className="wf-cap m-0">
          <code>{shown.code ?? "board::schema::invalid"}</code> on <code>{shown.path}</code>. An
          agent rewrites the record, and <code>board check {shown.path}</code> says what is wrong.
        </p>
      </div>
    );
  }
  return (
    <div className="wf-card" role="alert">
      <p className="m-0 font-medium">
        The check found {shown.problems.length}{" "}
        {shown.problems.length === 1 ? "problem" : "problems"}, so the graph is not drawn.
      </p>
      <ul className="m-0 flex flex-col gap-1.5 pl-4">
        {shown.problems.map((problem) => (
          <li key={`${problem.pointer}-${problem.code}`}>
            <code>{problem.pointer}</code>: {problem.message}{" "}
            <span className="wf-cap">
              <code>{problem.code}</code>
            </span>
          </li>
        ))}
      </ul>
      <p className="wf-cap m-0">
        An agent fixes the record and runs <code>board flightdeck check --path {shown.path}</code>.
      </p>
    </div>
  );
}

type DrawnProps = {
  graph: GraphRecord;
  shown: WorkflowShow;
  rows: GraphRow[];
  phone: boolean;
  initial: string | null;
  open: (path: string) => void;
};

/** The header, the source row, the switch and the graph of one checked graph. */
function Drawn({ graph, shown, rows, phone, initial, open }: DrawnProps) {
  const [hidden, setHidden] = useState<boolean | null>(null);
  const hideConditional = hidden ?? phone;
  const [chosen, setChosen] = useState<Selection>(() => selectionOf(graph, shown, initial));
  const [expanded, setExpanded] = useState<string | null>(() => {
    const first = selectionOf(graph, shown, initial);
    return first?.kind === "node" ? first.id : null;
  });
  const selection = stillThere(graph, chosen);
  const measure = useMemo(measurer, []);
  const family =
    typeof document === "undefined" ? "sans-serif" : getComputedStyle(document.body).fontFamily;
  const live = graph.bindings.length > 0;
  const liveAt = (id: string) =>
    live
      ? shown.bindings
          .filter((binding) => binding.target === id)
          .reduce((sum, binding) => sum + binding.records.length, 0)
      : 0;
  const placed = layout(graph, { hideConditional, liveAt, family, measure });
  const subjectIsFlow = graph.subject.ref.startsWith("/flows/");
  const older = shown.running !== null && shown.running !== graph.subject.ref;
  const sheet = phone || shown.limit !== null;
  const hide = (on: boolean) => {
    setHidden(on);
    const node = graph.nodes.find((each) => each.id === selection?.id);
    if (on && selection?.kind === "node" && node?.conditional) {
      setChosen(null);
    }
  };
  const by = graph.by.agent ?? graph.by.program ?? graph.by.person ?? "an agent";
  const readings = shown.readings;

  if (phone) {
    return (
      <div className="wf-page wf-phone">
        <div>
          <div className="wf-sub">Workflows</div>
          <h1>{graph.title}</h1>
          <div className="wf-sub" style={{ fontSize: 13 }}>
            {headline(graph, shown)}
          </div>
        </div>
        <Switch rows={rows} slug={shown.slug} phone />
        <HideToggle on={hideConditional} onChange={hide} />
        <RunSheet
          graph={graph}
          shown={shown}
          hideConditional={hideConditional}
          open={expanded}
          onOpen={setExpanded}
        />
      </div>
    );
  }

  return (
    <div className="wf-page">
      <div>
        <div className="wf-sub">Workflows</div>
        <div className="wf-row wf-between">
          <h1>{graph.title}</h1>
          <div className="wf-row">
            <button type="button" className="wf-ghost" onClick={() => open(graph.subject.ref)}>
              {subjectIsFlow ? "Open the flow record" : "Open the source"}
            </button>
          </div>
        </div>
        <div className="wf-sub">
          {graph.summary} {headline(graph, shown)}
        </div>
      </div>
      {older && shown.running !== null && (
        <div className="wf-note">
          <span>
            The changes plugin now runs <b>{graph.subject.ref.split("/")[2]}</b> at{" "}
            <code>{short(lastSegment(shown.running))}</code>. This graph reads{" "}
            <code>{short(lastSegment(graph.subject.ref))}</code>, so steps the new version adds or
            renames are missing here.
          </span>
        </div>
      )}
      <div className="wf-row wf-between">
        <Switch rows={rows} slug={shown.slug} phone={false} />
        <div className="wf-row">
          <HideToggle on={hideConditional} onChange={hide} />
        </div>
      </div>
      <div className="wf-row">
        <span className="wf-chip">
          Reads <code>{graph.subject.ref}</code>
        </span>
        <span className="wf-chip">
          at <code>{short(graph.subject.commit)}</code>
        </span>
        <FreshnessChip freshness={shown.freshness} older={older} />
        <span className="wf-cap">
          Read by {by} on {dayOf(graph.made)}.
          {readings !== null && (
            <>
              {" "}
              {countWord(readings)} earlier {readings === 1 ? "reading" : "readings"} in{" "}
              <code>board log</code>.
            </>
          )}
        </span>
      </div>
      {shown.limit !== null && (
        <div className="wf-note">
          <span>
            This graph has more than the page draws as a swimlane ({shown.limit}), so it shows as a
            run sheet.
          </span>
        </div>
      )}
      {sheet ? (
        <section className="wf-card" aria-label="Graph">
          <RunSheet
            graph={graph}
            shown={shown}
            hideConditional={hideConditional}
            open={expanded}
            onOpen={setExpanded}
          />
        </section>
      ) : (
        <div className="wf-split">
          <section className="wf-card wf-gcard" aria-label="Graph">
            <div className="wf-ghead">
              <Legend live={live} />
            </div>
            <div className="wf-gscroll">
              <Graph
                graph={graph}
                shown={shown}
                placed={placed}
                selection={selection}
                onSelect={setChosen}
              />
            </div>
          </section>
          <aside className="wf-card wf-panel" aria-label="Selected">
            <Panel
              graph={graph}
              shown={shown}
              selection={selection}
              onSelect={setChosen}
              open={open}
            />
          </aside>
        </div>
      )}
    </div>
  );
}

type Props = {
  /** The slug the address names, or null for the first graph. */
  slug: string | null;
  /** What the address selects, or null. */
  selected: string | null;
  /** Called when the server refuses the page's token. */
  onSignedOut: () => void;
};

/**
 * The Workflows page: one workflow graph from the board, drawn as a
 * swimlane beside a panel on desktop and as a run sheet on a phone, with
 * the open changes that its bindings place. It asks for the graph every
 * 15 seconds and the list every 30 seconds while the tab is visible, and
 * keeps the last graph it drew when a later ask fails.
 */
export default function Workflows({ slug, selected, onSignedOut }: Props) {
  const phone = usePhone();
  const list = useQuery(workflowsQuery());
  const rows = list.data?.data ?? [];
  const chosen = slug ?? rows.find((row) => row.status === "ok")?.slug ?? rows[0]?.slug ?? null;
  const shown = useQuery({ ...workflowQuery(chosen ?? ""), enabled: chosen !== null });
  const refused = isSignedOut(list.error) || isSignedOut(shown.error);
  const [copied, setCopied] = useState<string | null>(null);
  const [uncopyable, setUncopyable] = useState(false);

  useEffect(() => {
    if (refused) {
      onSignedOut();
    }
  }, [refused, onSignedOut]);

  const open = (path: string) => {
    const command = boardGet(path);
    setUncopyable(command === null);
    if (command === null) {
      setCopied(null);
      return;
    }
    navigator.clipboard?.writeText(command).then(
      () => setCopied(command),
      () => setCopied(null),
    );
  };

  const missing =
    shown.error instanceof ApiError &&
    shown.error.failure.kind === "status" &&
    shown.error.failure.status === 404;

  return (
    <>
      <p className="sr-only" aria-live="polite">
        {copied === null ? "" : `Copied ${copied} to the clipboard.`}
        {uncopyable ? uncopyableText : ""}
      </p>
      {uncopyable && <p className="wf-cap m-0 mb-2">{uncopyableText}</p>}
      {copied !== null && (
        <p className="wf-cap m-0 mb-2">
          Copied <code>{copied}</code> to the clipboard. Flightdeck has no record viewer yet.
        </p>
      )}
      {list.isPending && (
        <p aria-busy="true" className="text-muted-foreground m-0">
          Asking Board for the workflow graphs…
        </p>
      )}
      {list.isError && !refused && list.data === undefined && (
        <Failure error={list.error} onRetry={() => void list.refetch()} />
      )}
      {list.data !== undefined && rows.length === 0 && (
        <div className="wf-page">
          <h1>Workflows</h1>
          <div className="wf-card max-w-xl">
            <p className="m-0 font-medium">This board holds no workflow graph.</p>
            <p className="wf-cap m-0">
              An agent writes one with <code>board put /workflow-graphs/&lt;slug&gt;</code>, and it
              shows here.
            </p>
          </div>
        </div>
      )}
      {missing && (
        <div className="wf-page">
          <h1>Workflows</h1>
          <div className="wf-card max-w-xl" role="alert">
            <p className="m-0 font-medium">This board has no workflow graph named {chosen}.</p>
            <a
              className="text-primary"
              href={hrefOf({ view: "workflows", slug: null, selected: null })}
            >
              Show the first graph
            </a>
          </div>
        </div>
      )}
      {shown.isError && !refused && !missing && shown.data === undefined && (
        <Failure error={shown.error} onRetry={() => void shown.refetch()} />
      )}
      {shown.isError && shown.data !== undefined && (
        <p className="wf-cap m-0 mb-2" role="status">
          As of {new Date(shown.dataUpdatedAt).toLocaleTimeString("en-GB")}:{" "}
          {describeFailure(shown.error)}
        </p>
      )}
      {shown.data !== undefined &&
        (shown.data.graph === null || shown.data.status !== "ok" ? (
          <div className="wf-page">
            <div>
              <div className="wf-sub">Workflows</div>
              <h1>{rows.find((row) => row.slug === shown.data.slug)?.title ?? shown.data.slug}</h1>
            </div>
            <Switch rows={rows} slug={shown.data.slug} phone={phone} />
            <NotDrawn shown={shown.data} />
          </div>
        ) : (
          <Drawn
            key={`${shown.data.slug}-${phone}`}
            graph={shown.data.graph}
            shown={againstRunning(shown.data)}
            rows={rows}
            phone={phone}
            initial={slug === shown.data.slug ? selected : null}
            open={open}
          />
        ))}
    </>
  );
}

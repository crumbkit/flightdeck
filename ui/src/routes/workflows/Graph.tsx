import { cn } from "cn";
import { useId } from "react";
import { edgeKinds, type Graph as GraphRecord, type WorkflowShow } from "@/api/workflows";
import { channels, edgePath, type Layout, laneHead, type PlacedNode } from "@/lib/workflowLayout";
import { liveOf, type Selection } from "./model";
import { actorWord, Glyph, kindWord } from "./words";

/** Returns the words of an edge's label: its own, or its kind's word. */
function edgeLabel(kind: string, label: string | undefined): string {
  if (label !== undefined) {
    return label;
  }
  return { fail: "fails", retry: "retry", loop: "again" }[kind] ?? "";
}

/** What a group's kind is called under its label. */
const groupWord = { phase: "Phase", loop: "Loops", parallel: "Runs together" } as const;

/**
 * Returns where an edge's label sits: under the bottom right of `a` for an
 * edge back, beside the line between `a` and `b` in one lane, or above `b`.
 */
function labelAt(back: boolean, a: PlacedNode, b: PlacedNode) {
  if (back) {
    return { left: a.x + a.w - 4, top: a.y + a.h + 1 };
  }
  if (Math.abs(a.x - b.x) < 1) {
    return { left: a.x + a.w / 2 + 6, top: (a.y + a.h + b.y) / 2 - 8 };
  }
  return { left: b.x + b.w / 2 + 6, top: b.y - 26 };
}

type Props = {
  graph: GraphRecord;
  shown: WorkflowShow;
  placed: Layout;
  selection: Selection;
  onSelect: (selection: Selection) => void;
};

/**
 * The swimlane: a header per lane, a band per group with its label and
 * live count in the left gutter, the edges as SVG under the nodes, and one
 * button per node. Selecting a node thickens its edges and fades the nodes
 * it does not touch.
 */
export function Graph({ graph, shown, placed, selection, onSelect }: Props) {
  const uid = useId().replace(/[^a-zA-Z0-9-]/g, "");
  const at = new Map(placed.nodes.map((node) => [node.id, node]));
  const lane = new Map(graph.lanes.map((each) => [each.id, each]));
  const live = graph.bindings.length > 0;
  const chosen = selection?.kind === "node" ? selection.id : null;
  const channel = channels(placed);
  const toggle = (next: Selection) =>
    onSelect(selection?.kind === next?.kind && selection?.id === next?.id ? null : next);
  return (
    <div className="wf-graph" style={{ width: placed.width, height: placed.height }}>
      {placed.bands.map((band, index) => {
        const records = live ? liveOf(shown, band.id) : [];
        const old = records.filter((record) => !record.current).length;
        const on = selection?.kind === "group" && selection.id === band.id;
        return (
          <div key={band.id}>
            <div
              className={cn("wf-band", index % 2 === 1 && "wf-band-alt")}
              style={{ top: band.top, height: band.bottom - band.top }}
            />
            <button
              type="button"
              className="wf-bandlab"
              aria-pressed={on}
              style={{ top: band.top }}
              onClick={() => toggle({ kind: "group", id: band.id })}
            >
              <span className="wf-lab">{band.label}</span>
              <span className="wf-cap">{groupWord[band.kind]}</span>
              {records.length > 0 && (
                <>
                  <span className="wf-cap">
                    <b>{records.length}</b> open{old > 0 ? `, ${old} on an older flow` : ""}
                  </span>
                  <span className="wf-dots">
                    {records.map((record) => (
                      <i
                        key={record.path}
                        className={cn("wf-dot", !record.current && "wf-dot-old")}
                      />
                    ))}
                  </span>
                </>
              )}
            </button>
          </div>
        );
      })}
      {placed.lanes.map((each) => (
        <div key={each.id}>
          <div className="wf-lanehead" style={{ left: each.x, width: each.w }}>
            <b>{each.label}</b>
            <span>{actorWord[each.actor]}</span>
          </div>
          <div
            className="wf-laneline"
            style={{ left: each.x, top: laneHead, height: placed.height - laneHead }}
          />
        </div>
      ))}
      <svg width={placed.width} height={placed.height} aria-hidden="true">
        <defs>
          {edgeKinds.map((kind) => (
            <marker
              key={kind}
              id={`wf-ah-${kind}-${uid}`}
              viewBox="0 0 8 8"
              refX="7"
              refY="4"
              markerWidth="7"
              markerHeight="7"
              orient="auto-start-reverse"
            >
              <path className={`wf-ah-${kind}`} d="M0 0 8 4 0 8Z" />
            </marker>
          ))}
        </defs>
        {placed.edges.map((edge, index) => {
          const a = at.get(edge.from);
          const b = at.get(edge.to);
          if (a === undefined || b === undefined) {
            return null;
          }
          const touches = chosen !== null && (edge.from === chosen || edge.to === chosen);
          return (
            <path
              // biome-ignore lint/suspicious/noArrayIndexKey: two edges may join the same nodes
              key={index}
              className={cn(
                "wf-e",
                `wf-e-${edge.kind}`,
                touches && "wf-e-hi",
                chosen !== null && !touches && "wf-e-lo",
              )}
              d={edgePath(edge, a, b, placed, channel.get(edge) ?? 0)}
              markerEnd={`url(#wf-ah-${edge.kind}-${uid})`}
            />
          );
        })}
      </svg>
      {placed.edges.map((edge, index) => {
        const a = at.get(edge.from);
        const b = at.get(edge.to);
        const word = edgeLabel(edge.kind, edge.label);
        if (a === undefined || b === undefined || word === "") {
          return null;
        }
        const style = labelAt(placed.back.has(edge), a, b);
        return (
          <span
            // biome-ignore lint/suspicious/noArrayIndexKey: two edges may join the same nodes
            key={index}
            aria-hidden="true"
            className={cn("wf-elab", `wf-elab-${edge.kind}`)}
            style={style}
          >
            {word}
          </span>
        );
      })}
      {placed.nodes.map((node) => {
        const records = live ? liveOf(shown, node.id) : [];
        const near =
          chosen !== null &&
          (chosen === node.id ||
            placed.edges.some(
              (edge) =>
                (edge.from === chosen && edge.to === node.id) ||
                (edge.to === chosen && edge.from === node.id),
            ));
        const actor = lane.get(node.lane)?.actor ?? "agent";
        const tags = node.conditional || !node.blocks || records.length > 0;
        return (
          <button
            key={node.id}
            type="button"
            className={cn(
              "wf-node",
              `wf-node-${node.kind}`,
              `wf-node-${actor}`,
              node.conditional && "wf-node-cond",
              chosen !== null && !near && "wf-node-dim",
            )}
            aria-pressed={chosen === node.id}
            style={{ left: node.x, top: node.y, width: node.w, minHeight: node.h }}
            onClick={() => toggle({ kind: "node", id: node.id })}
          >
            <Glyph kind={node.kind} />
            <span className="wf-t">
              <span>
                <span className="sr-only">{kindWord[node.kind]}:</span> {node.label}
              </span>
              {tags && (
                <span className="wf-tags">
                  {node.conditional && <span className="wf-tag">if</span>}
                  {!node.blocks && <span className="wf-tag">warns only</span>}
                  {records.length > 0 && (
                    <span className="wf-livecount">{records.length} here</span>
                  )}
                </span>
              )}
            </span>
          </button>
        );
      })}
    </div>
  );
}

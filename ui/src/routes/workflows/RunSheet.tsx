import { cn } from "cn";
import type { Graph, WorkflowShow } from "@/api/workflows";
import { rankNodes, runSheet, visible } from "@/lib/workflowLayout";
import { liveOf } from "./model";
import { sourceOf } from "./Panel";
import { backWord, Glyph, lastSegment } from "./words";

type Props = {
  graph: Graph;
  shown: WorkflowShow;
  hideConditional: boolean;
  /** The id of the step whose detail is open, or null. */
  open: string | null;
  onOpen: (id: string | null) => void;
};

/**
 * The graph as a run sheet, top to bottom, for a phone: each group's
 * heading with its live count, then each step with its lane's label. A
 * move back to an earlier step is written under the step it leaves, and a
 * step's detail opens in place.
 */
export function RunSheet({ graph, shown, hideConditional, open, onOpen }: Props) {
  const { nodes, edges } = visible(graph, hideConditional);
  const { rank, back } = rankNodes(nodes, edges);
  const lane = new Map(graph.lanes.map((each) => [each.id, each]));
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const live = graph.bindings.length > 0;
  const sorted = runSheet(nodes, rank);
  return (
    <div className="wf-plist">
      {sorted.map((node, index) => {
        const previous = sorted[index - 1];
        const group =
          node.group !== undefined && (index === 0 || previous?.group !== node.group)
            ? graph.groups.find((each) => each.id === node.group)
            : undefined;
        const groupRecords = group !== undefined && live ? liveOf(shown, group.id) : [];
        const groupOld = groupRecords.filter((record) => !record.current).length;
        const actor = lane.get(node.lane)?.actor ?? "agent";
        const backs = edges.filter((edge) => edge.from === node.id && back.has(edge));
        const forward = edges.filter(
          (edge) =>
            edge.from === node.id &&
            !back.has(edge) &&
            (edge.kind === "fail" || edge.label !== undefined) &&
            byId.has(edge.to),
        );
        const records = live ? liveOf(shown, node.id) : [];
        const expanded = open === node.id;
        const source = node.cite === undefined ? undefined : sourceOf(graph, node.cite);
        return (
          <div key={node.id} className="contents">
            {group !== undefined && (
              <div className="wf-pgroup">
                <span className="wf-lab wf-pgroup-label">
                  {group.label}
                  {group.kind === "loop" ? " · loops" : ""}
                </span>
                {groupRecords.length > 0 && (
                  <span className="wf-cap">
                    {groupRecords.length} open{groupOld > 0 ? `, ${groupOld} older` : ""}
                  </span>
                )}
              </div>
            )}
            <button
              type="button"
              className={cn("wf-pnode", `wf-pnode-${node.kind}`, `wf-pnode-${actor}`)}
              aria-expanded={expanded}
              onClick={() => onOpen(expanded ? null : node.id)}
            >
              <span className="wf-spine">
                <Glyph kind={node.kind} />
              </span>
              <span>
                <span className="wf-lbl block">{node.label}</span>
                <span className="wf-meta">
                  <span>{lane.get(node.lane)?.label}</span>
                  {node.conditional && <span className="wf-chip">if</span>}
                  {!node.blocks && <span className="wf-chip wf-chip-amber">warns only</span>}
                  {records.length > 0 && (
                    <span className="wf-livecount">{records.length} here</span>
                  )}
                </span>
                {backs.map((edge) => (
                  <span
                    key={`${edge.kind}-${edge.to}`}
                    className={cn("wf-back block", `wf-back-${edge.kind}`)}
                  >
                    {backWord[edge.kind]} {byId.get(edge.to)?.label}
                    {edge.label === undefined ? "" : ` (${edge.label})`}
                  </span>
                ))}
                {forward.map((edge) => (
                  <span
                    key={`${edge.kind}-${edge.to}`}
                    className={cn("wf-back block", edge.kind === "fail" && "wf-back-fail")}
                  >
                    {edge.kind === "fail" ? "If it fails" : edge.label}: {byId.get(edge.to)?.label}
                  </span>
                ))}
                {expanded && (
                  <span className="wf-more">
                    {node.detail !== undefined && <span>{node.detail}</span>}
                    {node.conditional && <span>When: {node.when}</span>}
                    {node.cite !== undefined && source !== undefined && (
                      <span>
                        From <code className="wf-wrap">{lastSegment(source.ref)}</code>{" "}
                        {node.cite.at === "pointer" ? (
                          <code>{node.cite.pointer}</code>
                        ) : (
                          `lines ${node.cite.lines.from}–${node.cite.lines.to}`
                        )}
                      </span>
                    )}
                  </span>
                )}
              </span>
            </button>
          </div>
        );
      })}
    </div>
  );
}

import type { Cite, Graph, WorkflowShow } from "@/api/workflows";
import { liveOf, type Selection, waitsOnYou } from "./model";
import { actorWord, edgeWord, kindWord, lastSegment, short } from "./words";

/** Copies `board get <path>` for a record that Flightdeck cannot open. */
export type Open = (path: string) => void;

/** Returns the source that `cite` points into: the subject, then the sources. */
export function sourceOf(graph: Graph, cite: Cite) {
  return [graph.subject, ...graph.sources][cite.source];
}

/** The place in a source that a node or group comes from. */
function Comes({ graph, cite, open }: { graph: Graph; cite: Cite | undefined; open: Open }) {
  if (cite === undefined) {
    return null;
  }
  const source = sourceOf(graph, cite);
  if (source === undefined) {
    return null;
  }
  return (
    <>
      <span>Comes from</span>
      <span className="wf-wrap">
        <button type="button" className="wf-link" onClick={() => open(source.ref)}>
          <code>{source.ref}</code>
        </button>{" "}
        at{" "}
        {cite.at === "pointer" ? (
          <code>{cite.pointer}</code>
        ) : (
          `lines ${cite.lines.from}${cite.lines.to === cite.lines.from ? "" : `–${cite.lines.to}`}`
        )}
      </span>
    </>
  );
}

/** A live record: its title, which copies how to read it, and its version. */
function Record({
  path,
  title,
  version,
  current,
  open,
  same,
}: {
  path: string;
  title: string;
  version: string | null;
  current: boolean;
  open: Open;
  same: boolean;
}) {
  return (
    <div className="wf-item">
      <span>
        <button type="button" className="wf-link" onClick={() => open(path)}>
          {title}
        </button>
      </span>
      {!current && version !== null ? (
        <span className="wf-chip wf-chip-violet">flow {short(lastSegment(version))}</span>
      ) : (
        same && <span className="wf-cap">this version</span>
      )}
    </div>
  );
}

/** The counts of the records that a binding skipped or left out, when any. */
function Counts({ shown, target }: { shown: WorkflowShow; target: string }) {
  const states = shown.bindings.filter((binding) => binding.target === target);
  const skipped = states.reduce((sum, state) => sum + state.skipped, 0);
  const leftOut = states.reduce((sum, state) => sum + state.left_out, 0);
  const errors = states.flatMap((state) => (state.error === null ? [] : [state.error]));
  return (
    <>
      {skipped > 0 && (
        <p className="wf-cap">
          {skipped} {skipped === 1 ? "record was" : "records were"} skipped: degraded, or a field
          that is not text.
        </p>
      )}
      {leftOut > 0 && <p className="wf-cap">{leftOut} more past the first 500 are left out.</p>}
      {errors.map((error) => (
        <p key={error.message} className="wf-cap">
          Board could not list them: {error.code ?? error.message}
        </p>
      ))}
    </>
  );
}

type Props = {
  graph: Graph;
  shown: WorkflowShow;
  selection: Selection;
  onSelect: (selection: Selection) => void;
  open: Open;
};

/** The panel of the selected group: its steps, where it comes from and what is open in it. */
function GroupPanel({
  graph,
  shown,
  id,
  open,
}: Omit<Props, "selection" | "onSelect"> & { id: string }) {
  const group = graph.groups.find((each) => each.id === id);
  if (group === undefined) {
    return null;
  }
  const members = graph.nodes.filter((node) => node.group === group.id);
  const binding = graph.bindings.find((each) => each.target === group.id);
  const records = liveOf(shown, group.id);
  return (
    <>
      <div className="wf-row">
        <span className="wf-chip">{group.kind === "loop" ? "Loop" : "Phase"}</span>
        <span className="wf-cap">
          {members.length} steps, {members.filter((node) => node.kind === "gate").length} checks,{" "}
          {members.filter((node) => waitsOnYou(graph, node)).length} wait on you
        </span>
      </div>
      <h2>{group.label}</h2>
      <div className="wf-kv">
        <Comes graph={graph} cite={group.cite} open={open} />
      </div>
      {binding === undefined ? (
        <div className="wf-sec">
          <p className="wf-cap">
            This graph has no binding for {group.label}, so it shows no live runs here.
          </p>
        </div>
      ) : (
        <div className="wf-sec">
          <div className="wf-lab">Open here now</div>
          <p className="wf-cap">
            Changes under <code>{binding.under}</code> whose{" "}
            {binding.match.map((condition, index) => (
              <span key={condition.field}>
                {index > 0 && " and "}
                <code>{condition.field.slice(1)}</code>{" "}
                {condition.op === "equals" ? "is" : "is under"} <code>{condition.value}</code>
              </span>
            ))}
            .
          </p>
          <div>
            {records.length === 0 ? (
              <p className="wf-cap">Nothing is open in this phase.</p>
            ) : (
              records.map((record) => (
                <Record key={record.path} {...record} open={open} same={shown.running === null} />
              ))
            )}
          </div>
          <Counts shown={shown} target={group.id} />
        </div>
      )}
    </>
  );
}

/**
 * The panel beside the graph. A selected group shows its changes, those on
 * the graph's version first. A selected node, or the start when nothing is
 * selected, shows who acts, when it applies, where it comes from, where it
 * goes next, what comes back to it, what it reads or writes and its
 * changes.
 */
export function Panel({ graph, shown, selection, onSelect, open }: Props) {
  if (selection?.kind === "group") {
    return <GroupPanel graph={graph} shown={shown} id={selection.id} open={open} />;
  }
  const byId = new Map(graph.nodes.map((node) => [node.id, node]));
  const node =
    (selection === null ? undefined : byId.get(selection.id)) ??
    graph.nodes.find((each) => each.kind === "start");
  if (node === undefined) {
    return null;
  }
  const lane = graph.lanes.find((each) => each.id === node.lane);
  const group = graph.groups.find((each) => each.id === node.group);
  const out = graph.edges.filter((edge) => edge.from === node.id);
  const back = graph.edges.filter((edge) => edge.to === node.id && edge.kind !== "next");
  const records = graph.bindings.length > 0 ? liveOf(shown, node.id) : [];
  const go = (id: string) => onSelect({ kind: "node", id });
  return (
    <>
      <div className="wf-row">
        {waitsOnYou(graph, node) ? (
          <span className="wf-chip wf-chip-blue">Waits on you</span>
        ) : (
          <span className="wf-chip">{kindWord[node.kind]}</span>
        )}
        {node.conditional && <span className="wf-chip">Only some changes</span>}
        {node.kind === "gate" &&
          (node.blocks ? (
            <span className="wf-chip">Blocks</span>
          ) : (
            <span className="wf-chip wf-chip-amber">Warns only</span>
          ))}
      </div>
      <h2>{node.label}</h2>
      {node.detail !== undefined && <p>{node.detail}</p>}
      <div className="wf-kv">
        <span>Who</span>
        <span>
          {lane?.label}{" "}
          {lane !== undefined && <span className="wf-cap">({actorWord[lane.actor]})</span>}
        </span>
        {group !== undefined && (
          <>
            <span>In</span>
            <span>{group.label}</span>
          </>
        )}
        {node.conditional && (
          <>
            <span>When</span>
            <span>{node.when}</span>
          </>
        )}
        <Comes graph={graph} cite={node.cite} open={open} />
      </div>
      {out.length > 0 && (
        <div className="wf-sec">
          <div className="wf-lab">Next</div>
          {out.map((edge) => (
            <div key={`${edge.kind}-${edge.to}`} className="wf-item">
              <span>
                {edge.kind !== "next" && edge.kind !== "pass" && (
                  <span className="wf-cap">
                    {edgeWord[edge.kind]}
                    {edge.label === undefined ? "" : `, ${edge.label}`}:{" "}
                  </span>
                )}
                <button type="button" className="wf-link" onClick={() => go(edge.to)}>
                  {byId.get(edge.to)?.label ?? edge.to}
                </button>
                {edge.kind === "next" && edge.label !== undefined && (
                  <span className="wf-cap"> ({edge.label})</span>
                )}
                {edge.when !== undefined && <span className="wf-cap block">{edge.when}</span>}
              </span>
            </div>
          ))}
        </div>
      )}
      {back.length > 0 && (
        <div className="wf-sec">
          <div className="wf-lab">Comes back here from</div>
          {back.map((edge) => (
            <div key={`${edge.kind}-${edge.from}`} className="wf-item">
              <span>
                <button type="button" className="wf-link" onClick={() => go(edge.from)}>
                  {byId.get(edge.from)?.label ?? edge.from}
                </button>{" "}
                <span className="wf-cap">
                  {edgeWord[edge.kind]}
                  {edge.label === undefined ? "" : `, ${edge.label}`}
                </span>
              </span>
            </div>
          ))}
        </div>
      )}
      {node.refs.length > 0 && (
        <div className="wf-sec">
          <div className="wf-lab">Reads or writes</div>
          {node.refs.map((ref) => (
            <div key={ref} className="wf-item">
              <button type="button" className="wf-link wf-wrap" onClick={() => open(ref)}>
                <code>{ref}</code>
              </button>
            </div>
          ))}
        </div>
      )}
      {records.length > 0 && (
        <div className="wf-sec">
          <div className="wf-lab">Here now</div>
          {records.map((record) => (
            <Record key={record.path} {...record} open={open} same={false} />
          ))}
          <Counts shown={shown} target={node.id} />
        </div>
      )}
    </>
  );
}

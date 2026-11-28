import { describe, expect, it } from "vitest";
import type { Graph } from "@/api/workflows";
import { workflowGraph } from "@/api/workflows";
import development from "../../tests/fixtures/graphs/development-flow.json" with { type: "json" };
import weekly from "../../tests/fixtures/graphs/weekly-suite-loop.json" with { type: "json" };
import { channels, edgePath, layout, rankNodes, runSheet, visible } from "./workflowLayout";

/** Measures 7 px a character, as a monospace face would. */
const measure = (text: string) => text.length * 7;

function laid(graph: Graph, hideConditional = false) {
  return layout(graph, { hideConditional, liveAt: () => 0, family: "sans-serif", measure });
}

const developmentFlow = workflowGraph.parse(development);
const weeklyLoop = workflowGraph.parse(weekly);

describe("layout", () => {
  it("puts each node below every node that reaches it by a forward edge", () => {
    for (const graph of [developmentFlow, weeklyLoop]) {
      const placed = laid(graph);
      const at = new Map(placed.nodes.map((node) => [node.id, node]));
      for (const edge of placed.edges) {
        if (placed.back.has(edge)) {
          continue;
        }
        const a = at.get(edge.from);
        const b = at.get(edge.to);
        expect(b?.y, `${edge.from} -> ${edge.to}`).toBeGreaterThanOrEqual(
          (a?.y ?? 0) + (a?.h ?? 0),
        );
      }
    }
  });

  it("routes fail, retry and loop edges that run back up the right-hand channel", () => {
    const placed = laid(developmentFlow);
    const backs = placed.edges.filter((edge) => placed.back.has(edge));
    expect(backs.map((edge) => `${edge.from}>${edge.to}:${edge.kind}`)).toEqual([
      "owner-approval>write-plan:fail",
      "suite-run>build-work:fail",
      "review-run>build-work:retry",
      "paths>write-plan:retry",
      "paused>build-work:next",
    ]);
    const channel = channels(placed);
    const at = new Map(placed.nodes.map((node) => [node.id, node]));
    for (const edge of backs) {
      const x = channel.get(edge) ?? 0;
      expect(x).toBeGreaterThan(placed.lanesRight);
      expect(x).toBeLessThanOrEqual(placed.width);
      const a = at.get(edge.from);
      const b = at.get(edge.to);
      if (a !== undefined && b !== undefined) {
        expect(edgePath(edge, a, b, placed, x)).toContain(`H${x - 6}`);
      }
    }
    const loop = laid(weeklyLoop);
    expect(loop.edges.filter((edge) => loop.back.has(edge)).map((edge) => edge.kind)).toEqual([
      "loop",
    ]);
  });

  it("gives each lane a column and each group with a node a band around its nodes", () => {
    const placed = laid(developmentFlow);
    expect(placed.lanes.map((lane) => lane.id)).toEqual([
      "lead",
      "owner",
      "builder",
      "reviewer",
      "board",
    ]);
    for (const band of placed.bands) {
      for (const node of placed.nodes.filter((each) => each.group === band.id)) {
        expect(node.y).toBeGreaterThan(band.top);
        expect(node.y + node.h).toBeLessThan(band.bottom);
      }
    }
    const lanes = new Map(placed.lanes.map((lane) => [lane.id, lane]));
    for (const node of placed.nodes) {
      const lane = lanes.get(node.lane);
      expect(node.x).toBe((lane?.x ?? 0) + 7);
    }
  });

  it("never puts two nodes of one lane at one rank", () => {
    const { rank } = rankNodes(developmentFlow.nodes, developmentFlow.edges);
    const seen = new Set<string>();
    for (const node of developmentFlow.nodes) {
      const key = `${node.lane}:${rank.get(node.id)}`;
      expect(seen.has(key), key).toBe(false);
      seen.add(key);
    }
  });
});

describe("visible", () => {
  it("hides conditional checks and joins each removed node's neighbours", () => {
    const shown = visible(developmentFlow, true);

    expect(shown.nodes.some((node) => node.conditional)).toBe(false);
    expect(shown.nodes).toHaveLength(
      developmentFlow.nodes.filter((node) => !node.conditional).length,
    );
    // route -> mockups -> mockup-versions -> data-signed -> lifecycle-declared
    // -> lifecycle-record -> owner-approval are all conditional between
    // route and owner-approval.
    expect(shown.edges).toContainEqual({ from: "route", to: "owner-approval", kind: "next" });
    const ids = new Set(shown.nodes.map((node) => node.id));
    for (const edge of shown.edges) {
      expect(ids.has(edge.from) && ids.has(edge.to), `${edge.from} -> ${edge.to}`).toBe(true);
    }
    expect(visible(developmentFlow, false).nodes).toBe(developmentFlow.nodes);
  });
});

describe("runSheet", () => {
  it("keeps each group's steps together, in the order of their ranks", () => {
    const { nodes, edges } = visible(developmentFlow, true);
    const { rank } = rankNodes(nodes, edges);
    const order = runSheet(nodes, rank).map((node) => node.group ?? "-");
    const runs = order.filter((group, index) => group !== order[index - 1]);
    expect(runs.filter((group) => group !== "-")).toEqual(["plan", "build", "review", "land"]);
  });
});

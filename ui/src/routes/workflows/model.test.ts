import { describe, expect, it } from "vitest";
import type { Placed, WorkflowShow } from "@/api/workflows";
import { againstRunning, phoneLabel, phoneLabels } from "./model";

const graphVersion = "/flows/development/3029fae590f5a628";
const olderVersion = "/flows/development/23eb709b901bc3e3";
const runningVersion = "/flows/development/7c1e04b29d3a5f60";

/** Returns a change on `version`, marked as the server marks it against the graph's version. */
function change(path: string, version: string | null): Placed {
  return { path, title: path, version, current: version === null || version === graphVersion };
}

/** Returns a page document whose plan binding places `records`, with `running` as the running version. */
function shownWith(records: Placed[], running: string | null): WorkflowShow {
  return {
    slug: "development-flow",
    path: "/workflow-graphs/development-flow",
    status: "ok",
    code: null,
    graph: null,
    problems: [],
    limit: null,
    bindings: [{ target: "plan", records, left_out: 0, skipped: 0, error: null }],
    readings: null,
    freshness: { state: "current" },
    running,
  };
}

describe("againstRunning", () => {
  const records = [
    change("/changes/on-the-graphs-version", graphVersion),
    change("/changes/on-an-older-version", olderVersion),
    change("/changes/on-the-running-version", runningVersion),
    change("/changes/with-no-version", null),
  ];

  it("leaves the changes as the server marked them while no version runs", () => {
    const shown = shownWith(records, null);

    expect(againstRunning(shown)).toBe(shown);
  });

  it("marks every change not on the running version as older, including the graph's own", () => {
    const shown = againstRunning(shownWith(records, runningVersion));

    expect(
      shown.bindings
        .flatMap((binding) => binding.records)
        .map((record) => [record.path, record.current]),
    ).toEqual([
      ["/changes/on-the-graphs-version", false],
      ["/changes/on-an-older-version", false],
      ["/changes/on-the-running-version", true],
      ["/changes/with-no-version", true],
    ]);
  });
});

describe("phoneLabel", () => {
  it("drops a trailing flow and keeps the first and last word of a longer title", () => {
    expect(phoneLabel("Development flow")).toBe("Development");
    expect(phoneLabel("Small flow")).toBe("Small");
    expect(phoneLabel("Weekly suite loop")).toBe("Weekly loop");
    expect(phoneLabel("broken")).toBe("broken");
  });
});

describe("phoneLabels", () => {
  it("shortens each title when the short labels differ", () => {
    expect(phoneLabels(["Development flow", "Small flow", "Weekly suite loop"])).toEqual([
      "Development",
      "Small",
      "Weekly loop",
    ]);
  });

  it("keeps the full titles of the graphs whose short labels are equal", () => {
    expect(phoneLabels(["Small flow", "Weekly suite loop", "Weekly review loop"])).toEqual([
      "Small",
      "Weekly suite loop",
      "Weekly review loop",
    ]);
  });
});

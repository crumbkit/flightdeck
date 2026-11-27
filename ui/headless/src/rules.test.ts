import { describe, expect, it } from "vitest";
import { CHECKERS, rulesAddress, rulesView } from "./rules.ts";
import {
  branchRun,
  fixtureDocument,
  withColumn,
  withoutRepository,
  withPassing,
  withStaleReport,
  withSupersededReport,
} from "./testDocuments.ts";

const all = rulesAddress.parse("");

describe("rulesView of the fixture (artboards 52 and 53)", () => {
  const view = rulesView(fixtureDocument(), all);

  it("says how many rules reach how many repositories, and how many fail", () => {
    expect(view.headline).toBe("6 rules reach 4 repositories, and 3 of them fail");
  });

  it("counts the rules in each tile", () => {
    expect(view.tiles.map((tile) => [tile.label, tile.count])).toEqual([
      ["Failing", 3],
      ["Stale", 0],
      ["Current", 0],
      ["Stated, not measured", 2],
    ]);
  });

  it("sorts the rules failing first, then by name", () => {
    expect(view.rules.map((rule) => [rule.id, rule.word])).toEqual([
      ["/rules/rust/file-size", "failing"],
      ["/rules/payments/v1-signer", "failing"],
      ["/rules/toolchains/rust", "failing"],
      ["/rules/writing/plain", "not readable"],
      ["/rules/app/handlers", "stated"],
      ["/rules/rust/server", "stated"],
    ]);
  });

  it("captions each rule with its first failure or what makes it stated", () => {
    const caption = (id: string) => view.rules.find((rule) => rule.id === id)?.caption;
    expect(caption("/rules/rust/file-size")).toBe("owner/api: 1 file over");
    expect(caption("/rules/toolchains/rust")).toBe("owner/api pins 1.90.0");
    expect(caption("/rules/payments/v1-signer")).toBe("owner/app: 2 left");
    expect(caption("/rules/rust/server")).toBe("judged by an audit");
    expect(caption("/rules/writing/plain")).toBe("board::ref::dangling");
  });

  it("names the repositories a rule reaches", () => {
    const reach = (id: string) => view.rules.find((rule) => rule.id === id)?.repositories;
    expect(reach("/rules/rust/file-size")).toBe("all 3 with a report");
    expect(reach("/rules/app/handlers")).toBe("owner/app");
  });

  it("lists each repository's report, its drift and the rules it adds", () => {
    expect(
      view.reports.map((report) => [
        report.name,
        report.word,
        report.drift,
        report.driftItems,
        report.adds,
      ]),
    ).toEqual([
      [
        "owner/app",
        "current",
        "drifts",
        ["Overrides /rules/rust/server: requirement", "Lacks /rules/go/vet"],
        ["/rules/app/handlers"],
      ],
      ["owner/api", "current", "drifts", ["Lacks /rules/go/vet"], []],
      ["owner/ci", "current", "drifts", ["Lacks /rules/go/vet"], []],
      ["owner/gone", "no report", "unknown", [], null],
    ]);
    expect(view.reports[2]).toMatchObject({
      run: "run #7",
      commit: "84016e5",
      age: "59 min",
      rules: 5,
    });
    expect(view.reports[3]?.note).toBe("No report, so its drift is unknown.");
  });

  it("sums the imposed set and how many repositories drift from it", () => {
    expect(view.imposed).toMatchObject({ name: "controls", count: 5, drifting: 3 });
    expect(view.imposed.toggle).toBe("Show the 5 imposed rules");
    expect(view.reportsCurrent).toBe(3);
  });

  it("counts the rules by the five checkers the Rules artboard lists, by when an agent is shown them and by kind", () => {
    expect(CHECKERS).toEqual(["budget", "version", "pattern", "judged", "written"]);
    expect(view.checkers.map((entry) => [entry.key, entry.count])).toEqual([
      ["budget", 1],
      ["version", 1],
      ["judged", 1],
      ["written", 2],
    ]);
    expect(view.shown.map((entry) => [entry.key, entry.count])).toEqual([
      ["edit", 3],
      ["failing", 3],
    ]);
    expect(view.kinds.map((entry) => [entry.key, entry.count])).toEqual([["standing", 6]]);
  });
});

describe("rulesView's filters and sort, held in the address", () => {
  it("keeps only the rules of the chosen checker and state", () => {
    const view = rulesView(fixtureDocument(), rulesAddress.parse("?checker=written"));
    expect(view.rules.map((rule) => rule.id)).toEqual([
      "/rules/app/handlers",
      "/rules/rust/server",
    ]);
    const notCurrent = rulesView(
      fixtureDocument(),
      rulesAddress.parse("?state=not_current&shown=edit"),
    );
    expect(notCurrent.rules.map((rule) => rule.id)).toEqual(["/rules/writing/plain"]);
    expect(rulesAddress.parse("?checker=zero").checker).toBe("all");
  });

  it("names what a pattern rule quotes beside its checker", () => {
    const document = fixtureDocument();
    const rows = document.rows.map((row) =>
      row.id === "/rules/rust/server"
        ? {
            ...row,
            checker: { check: "pattern", file: "/rules/rust/server.patterns", quote: "match-only" },
          }
        : row,
    );
    const rule = rulesView({ ...document, rows }, all).rules.find(
      (each) => each.id === "/rules/rust/server",
    );
    expect(rule?.checker).toBe("pattern · match-only");
  });

  it("sorts by name when asked, in either direction", () => {
    const view = rulesView(
      fixtureDocument(),
      rulesAddress.parse("?sort=rule&direction=descending"),
    );
    expect(view.rules[0]?.name).toBe("Write the README plainly");
    expect(view.sort).toEqual({ key: "rule", direction: "descending" });
  });

  it("writes only what differs from the defaults", () => {
    expect(rulesAddress.format({ ...all, kind: "obligation" })).toBe("?kind=obligation");
    expect(rulesAddress.format(all)).toBe("");
  });

  it("keeps a failing rule failing when another repository's report is stale", () => {
    const view = rulesView(withStaleReport("owner/ci", 9), all);
    expect(view.rules.find((rule) => rule.id === "/rules/rust/file-size")?.word).toBe("failing");
    expect(view.reports[2]?.word).toBe("stale");
  });

  it("reads a rule that passes everywhere but a stale report as stale, and counts it in the Stale tile", () => {
    const passing = withPassing("owner/api", "/rules/rust/file-size");
    const view = rulesView(withStaleReport("owner/ci", 9, passing), all);
    const size = view.rules.find((rule) => rule.id === "/rules/rust/file-size");
    expect(size?.word).toBe("stale");
    expect(size?.caption).toBe("owner/ci report is 9 d old");
    expect(view.tiles.find((tile) => tile.key === "stale")?.count).toBe(1);
  });

  it("reads a rule by its readable reports alone, so a repository with no report leaves it current", () => {
    const passing = withPassing("owner/api", "/rules/rust/file-size");
    const view = rulesView(passing, all);
    const size = view.rules.find((rule) => rule.id === "/rules/rust/file-size");
    expect(size?.word).toBe("current");
    expect(size?.reached).not.toContain("owner/gone");
  });

  it("says that a rule is stale when a repository it reaches has a stale report or none, and that an unreadable rule is in no tile", () => {
    expect(rulesView(fixtureDocument(), all).note).toBe(
      "A measured rule fails when any repository's report fails it, and is stale when a repository it reaches has a stale report or none. A rule a report could not read is in no tile. Pattern, judged and written rules are stated: a report lists them and measures nothing.",
    );
  });
});

describe("rulesView of a document with the fetch's runs (artboards 52 and 53)", () => {
  const DAY = 86_400;

  it("gives a stale report's source run the sentence of its job's interval and its newest run's age", () => {
    const stale = withStaleReport("owner/ci", 9);
    const document = withColumn(
      "owner/ci",
      { every_hours: 24, newest_run: branchRun("owner/ci", 7, 9 * DAY + 60, true, stale) },
      stale,
    );
    const ci = rulesView(document, all).reports[2];
    expect(ci).toMatchObject({ run: "run #7", branch: "main" });
    expect(ci?.runNote).toBe("Its controls job runs daily, and the newest main run is 9 days old.");
    expect(ci?.sentence).toBe(ci?.runNote);
    const unpaced = withColumn("owner/ci", { every_hours: null }, document);
    expect(rulesView(unpaced, all).reports[2]?.runNote).toBe("The newest main run is 9 days old.");
  });

  it("names the newest run, its commit and the earlier report's rules of a repository whose newest run uploaded none", () => {
    const document = withSupersededReport("owner/app");
    const app = rulesView(document, all).reports[0];
    expect(app).toMatchObject({
      run: "run #12",
      commit: "0012e19",
      age: null,
      rules: 7,
      word: "no report",
      runUrl: "https://github.com/owner/app/actions/runs/9012",
      runNote: "finished without uploading a controls report",
      sentence: "Main run #12 finished without uploading a controls report.",
    });
  });

  it("reads a rule that only a repository with an earlier report reaches as stale, because it sent no report", () => {
    const alone = withoutRepository("owner/ci", withoutRepository("owner/api"));
    const view = rulesView(withSupersededReport("owner/app", alone), all);
    const size = view.rules.find((rule) => rule.id === "/rules/rust/file-size");
    expect(size).toMatchObject({
      word: "stale",
      caption: "owner/app sent no report",
      reached: ["owner/app"],
    });
    expect(view.rules.find((rule) => rule.id === "/rules/rust/server")?.word).toBe("stated");
  });
});

import { describe, expect, it } from "vitest";
import { complianceAddress, complianceView } from "./compliance.ts";
import {
  fixtureDocument,
  withInvalidReport,
  withoutRepository,
  withStaleReport,
  withSuiteWithoutRun,
} from "./testDocuments.ts";

const every = complianceAddress.parse("");

describe("complianceView of the fixture (artboards 46, 54 and 56)", () => {
  const view = complianceView(fixtureDocument(), every);

  it("says what fails, in how many repositories, and which sent no report", () => {
    expect(view.headline).toBe("6 checks fail in 3 repositories, and 1 repository sent no report");
  });

  it("gives each tile its count and the list it links to", () => {
    expect(view.tiles).toEqual([
      { key: "failing", label: "Failing", count: 6, word: "failing", target: "failing" },
      { key: "stale", label: "Stale", count: 0, word: "stale", target: "stale" },
      { key: "current", label: "Current", count: 5, word: "current", target: null },
      { key: "no_report", label: "No report", count: 1, word: "no report", target: "missing" },
    ]);
  });

  it("heads each column with its report's age, or says it has none", () => {
    expect(view.columns.map((column) => column.caption)).toEqual([
      "report 1 h",
      "report 1 h",
      "report 59 min",
      "no report",
    ]);
    expect(view.columns[3]?.spoken).toBe("owner/gone, no report");
  });

  it("puts measured rules, zero rules and suites in sections, and holds written rules apart", () => {
    expect(
      view.sections.map((section) => [section.key, section.rows.map((row) => row.id)]),
    ).toEqual([
      ["measured", ["/rules/rust/file-size", "/rules/toolchains/rust"]],
      ["zero", ["/rules/payments/v1-signer"]],
      ["suite", ["/suites/<repo>-rust-standard"]],
    ]);
    expect(view.written.rows).toHaveLength(4);
    expect(view.written.toggle).toBe("Show 4 written rules");
  });

  it("joins each repository's own suite into one row, as the matrix shows it", () => {
    const suite = view.sections[2]?.rows[0];
    expect(suite?.name).toBe("Rust standard audit");
    expect(suite?.checker).toBe("<repo>-rust-standard");
    expect(suite?.cells.map((cell) => [cell.word, cell.caption])).toEqual([
      ["current", "4 d · +1"],
      ["failing", "1 d · +1"],
      ["–", null],
      ["no report", null],
    ]);
  });

  it("captions a failing budget, pin and zero rule with what they measured", () => {
    const [size, rust] = view.sections[0]?.rows ?? [];
    expect(size?.cells[1]).toMatchObject({ word: "failing", caption: "1 over" });
    expect(rust?.cells[1]).toMatchObject({ word: "failing", caption: "1.90.0" });
    expect(view.sections[1]?.rows[0]?.cells.map((cell) => cell.caption)).toEqual([
      "2 left",
      "2 left",
      "2 left",
      null,
    ]);
  });

  it("gives every cell a spoken name", () => {
    const rows = [...view.sections.flatMap((section) => section.rows), ...view.written.rows];
    for (const row of rows) {
      for (const cell of row.cells) expect(cell.spoken).toMatch(/^owner\/\w+, .+: .+/);
    }
    expect(view.sections[0]?.rows[0]?.cells[1]?.spoken).toBe(
      "owner/api, Keep each Rust file under 60 bytes: failing, 1 over",
    );
    expect(view.sections[2]?.rows[0]?.cells[2]?.spoken).toBe(
      "owner/ci, Rust standard audit: does not apply",
    );
  });

  it("lists failures rules first, then suites, joining repositories that fail alike", () => {
    expect(view.failing.count).toBe(6);
    expect(view.failing.items.map((item) => [item.repositories, item.detail])).toEqual([
      [["owner/api"], "/src/main.rs is 67 bytes, and the limit is 60."],
      [["owner/api"], "/rust-toolchain.toml line 1 pins 1.90.0, and the rule needs at least 1.99."],
      [["owner/app", "owner/api", "owner/ci"], "The run 1 d ago reports remaining 2."],
      [["owner/api"], "The run at 0e7502e, 1 day ago, failed 1 case: RUST-7."],
    ]);
    expect(view.failing.items[2]?.run).toBeNull();
  });

  it("names the repositories without a current report", () => {
    expect(view.missing.items).toEqual([
      {
        key: "owner/gone",
        repository: "owner/gone",
        word: "no report",
        detail: "No report was found for it, so its rules and suites have no state.",
        run: null,
      },
    ]);
  });

  it("says when the snapshot was built and when the next build runs", () => {
    expect(view.snapshot.sentence).toBe(
      "This page is a snapshot built at 23:13 UTC on 25 November 2026. It shows each repository's newest report at that time, and the next build is at 23:13 UTC tomorrow.",
    );
    expect(view.snapshot.buildRun).toEqual({ label: "Build run #412", url: null });
  });

  it("sums each repository for the phone's list", () => {
    expect(
      view.repositories.map((summary) => [summary.name, summary.summary, summary.word]),
    ).toEqual([
      ["owner/app", "1 failing · report 1 h", "failing"],
      ["owner/api", "4 failing · report 1 h", "failing"],
      ["owner/ci", "1 failing · report 59 min", "failing"],
      ["owner/gone", "no report", "no report"],
    ]);
  });
});

describe("complianceView's filter", () => {
  it("shows only rows with a cell that is not current when Not current is pressed", () => {
    const document = withoutRepository("owner/gone");
    const all = complianceView(document, every);
    const view = complianceView(document, complianceAddress.parse("?show=not_current"));
    expect(view.show).toBe("not_current");
    expect(all.written.rows.map((row) => row.key)).toContain("/rules/rust/server imposed");
    expect(view.written.rows.map((row) => row.key)).toEqual([
      "/rules/app/handlers repository",
      "/rules/rust/server repository",
      "/rules/writing/plain imposed",
    ]);
    expect(view.sections.flatMap((section) => section.rows)).toHaveLength(4);
  });

  it("reads an address it does not know as every state", () => {
    expect(complianceAddress.parse("?show=nonsense").show).toBe("every");
    expect(complianceAddress.format({ show: "every" })).toBe("");
    expect(complianceAddress.format({ show: "not_current" })).toBe("?show=not_current");
  });
});

describe("complianceView with stale and missing reports", () => {
  it("lists a stale report's rules once for its repository, and the report among the missing", () => {
    const view = complianceView(withStaleReport("owner/ci", 9), every);
    expect(view.headline).toBe(
      "6 checks fail in 3 repositories, 2 are stale, and 1 repository sent no report",
    );
    expect(view.stale.count).toBe(2);
    expect(view.stale.items.map((item) => [item.repositories, item.name, item.detail])).toEqual([
      [
        ["owner/ci"],
        "2 measured rules",
        "Its report is 9 days old, so the file size and rust rules are stale.",
      ],
    ]);
    expect(view.missing.items.map((item) => [item.repository, item.word, item.detail])).toEqual([
      ["owner/ci", "report 9 d old", "Its newest report was made 9 days before the snapshot."],
      [
        "owner/gone",
        "no report",
        "No report was found for it, so its rules and suites have no state.",
      ],
    ]);
    expect(view.columns[2]).toMatchObject({ caption: "report 9 d", stale: true });
  });

  it("puts a suite that never ran first among the stale, as the oldest", () => {
    const stale = withSuiteWithoutRun("/suites/app-rust-standard", "owner/app");
    const view = complianceView(stale, every);
    expect(view.stale.items[0]).toMatchObject({
      repositories: ["owner/app"],
      name: "Rust standard audit",
      detail: "The suite /suites/app-rust-standard has no clean full run yet.",
    });
  });

  it("says why a report is invalid", () => {
    const view = complianceView(withInvalidReport("owner/ci", "rules[0].result"), every);
    expect(view.missing.items[0]).toMatchObject({
      repository: "owner/ci",
      word: "invalid report",
      detail: "Its report is not one this build reads, at rules[0].result: unknown variant.",
    });
    expect(view.columns[2]?.caption).toBe("invalid report");
  });
});

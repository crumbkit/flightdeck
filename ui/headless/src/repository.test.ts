import { describe, expect, it } from "vitest";
import { repositoryAddress, repositoryView } from "./repository.ts";
import {
  branchRun,
  fixtureDocument,
  withColumn,
  withPassing,
  withStaleReport,
  withSupersededReport,
} from "./testDocuments.ts";

const none = repositoryAddress.parse("");

describe("repositoryView of one repository (artboards 47, 55 and 56)", () => {
  const view = repositoryView(fixtureDocument(), "owner/api", none);

  it("says what the repository fails and where its report came from", () => {
    expect(view?.headline).toBe("owner/api fails 3 rules and 1 audit");
    expect(view?.subtitle).toBe("Report at 2d9c956, 1 h before the snapshot");
    expect(repositoryView(fixtureDocument(), "owner/ci", none)?.subtitle).toBe(
      "Report from run #7 at 84016e5, 59 min before the snapshot",
    );
  });

  it("lists the measured rules with a budget's long checker, a version rule's short one, and what they measured", () => {
    expect(
      view?.measured.rows.map((row) => [row.name, row.checker, row.measured, row.word]),
    ).toEqual([
      [
        "Keep each Rust file under 60 bytes",
        "budget · bytes per file · max 60",
        "1 file over: src/main.rs 67",
        "failing",
      ],
      ["Pin Rust 1.99 or later", "version", "rust-toolchain.toml pins 1.90.0", "failing"],
    ]);
  });

  it("lists a failing measured rule before those that pass, keeping the document's order otherwise", () => {
    const passing = withPassing("owner/api", "/rules/rust/file-size");
    const api = repositoryView(passing, "owner/api", none);
    expect(api?.measured.rows.map((row) => [row.name, row.word])).toEqual([
      ["Pin Rust 1.99 or later", "failing"],
      ["Keep each Rust file under 60 bytes", "current"],
    ]);
  });

  it("sets the paths it measured in the mono face", () => {
    expect(view?.measured.rows.map((row) => row.measuredPhrase)).toEqual([
      ["1 file over: ", { mono: "src/main.rs" }, " 67"],
      [{ mono: "rust-toolchain.toml" }, " pins 1.90.0"],
    ]);
  });

  it("names the suite that judges its written rules and the age of that suite's run", () => {
    expect(view?.written.caption).toBe("Judged by the Rust standard audit, 1 d ago");
  });

  it("heads the failing panel's table with the budget's unit and gives it no Checker or Reads row", () => {
    expect(view?.selected?.overHeader).toBe("Bytes");
    const terms = view?.selected?.fields.map((field) => field.term);
    expect(terms).not.toContain("Reads");
    expect(terms).not.toContain("Checker");
    const measuredAt = view?.selected?.fields.find((field) => field.term === "Measured at");
    expect(measuredAt?.phrase).toEqual([{ mono: "2d9c956" }]);
  });

  it("lists the zero rules with their suite and newest run", () => {
    expect(view?.zero.rows.map((row) => [row.suite, row.measured, row.word])).toEqual([
      ["/suites/signer-callers", "remaining 2 · 1 d old, and the rule allows 7 d", "failing"],
    ]);
  });

  it("lists only the suites that name the repository, with their run and result", () => {
    expect(view?.suites.rows.map((row) => [row.id, row.run, row.commitsSince, row.result])).toEqual(
      [["/suites/api-rust-standard", "1 d ago at 0e7502e", 1, "1 case failed: RUST-7"]],
    );
  });

  it("lists the written rules that reach it", () => {
    expect(view?.written.rows.map((row) => [row.name, row.word])).toEqual([
      ["Name each server handler after its route", "written"],
      ["Write the README plainly", "not readable"],
    ]);
  });

  it("opens the first failing rule when the address names none", () => {
    expect(view?.selected?.id).toBe("/rules/rust/file-size");
    expect(view?.selected?.label).toBe("Failing · Keep each Rust file under 60 bytes");
    expect(view?.selected?.over).toEqual([{ file: "/src/main.rs", used: 67 }]);
    expect(view?.selected?.recheck.argv).toEqual([
      "board",
      "rules",
      "report",
      "--path",
      "/repos/api",
    ]);
  });

  it("opens the rule the address names, and ignores one that does not reach the repository", () => {
    const named = repositoryView(
      fixtureDocument(),
      "owner/api",
      repositoryAddress.parse("?rule=%2Frules%2Ftoolchains%2Frust"),
    );
    expect(named?.selected?.id).toBe("/rules/toolchains/rust");
    expect(named?.selected?.behind).toEqual([
      { file: "/rust-toolchain.toml", line: 1, found: "1.90.0" },
    ]);
    const elsewhere = repositoryView(
      fixtureDocument(),
      "owner/api",
      repositoryAddress.parse("?rule=%2Frules%2Fapp%2Fhandlers"),
    );
    expect(elsewhere?.selected?.id).toBe("/rules/rust/file-size");
  });

  it("puts failing checks first in the phone's one list", () => {
    expect(view?.checks.map((check) => [check.name, check.word])).toEqual([
      ["Keep each Rust file under 60 bytes", "failing"],
      ["Pin Rust 1.99 or later", "failing"],
      ["No service still calls the v1 webhook signer", "failing"],
      ["Rust standard audit", "failing"],
    ]);
  });

  it("explains each state it shows, with the document's window", () => {
    expect(view?.legend.map((entry) => entry.word)).toEqual([
      "current",
      "stale",
      "failing",
      "no report",
    ]);
    expect(view?.legend[1]?.meaning).toBe(
      "The newest report or clean run is older than 7 days, or no clean run exists.",
    );
  });

  it("names every line's state for a screen reader", () => {
    for (const row of view?.measured.rows ?? []) expect(row.spoken).toMatch(/: failing|: current/);
  });

  it("returns null for a repository the document does not hold", () => {
    expect(repositoryView(fixtureDocument(), "owner/nowhere", none)).toBeNull();
  });

  it("says a repository sent no report, and opens nothing", () => {
    const gone = repositoryView(fixtureDocument(), "owner/gone", none);
    expect(gone?.headline).toBe("owner/gone sent no report");
    expect(gone?.selected).toBeNull();
  });

  it("counts a stale report's checks", () => {
    const ci = repositoryView(withStaleReport("owner/ci", 9), "owner/ci", none);
    expect(ci?.headline).toBe("owner/ci fails 1 rule, and 2 of its checks are stale");
  });
});

describe("repositoryView of a document with the fetch's runs (artboard 55)", () => {
  it("says how many commits the branch has had since the report's run", () => {
    const count = (commits: number | null) =>
      repositoryView(
        withColumn("owner/ci", {
          commits_since: commits,
          report_run: branchRun("owner/ci", 7, 3600, true),
        }),
        "owner/ci",
        none,
      )?.commitsSince;
    expect([count(3), count(1), count(0), count(null)]).toEqual([
      "3 commits on main since that run",
      "1 commit on main since that run",
      "no commit on main since that run",
      null,
    ]);
  });

  it("lists the globs a rule applies to after its purpose, in the mono face", () => {
    const document = fixtureDocument();
    const rows = document.rows.map((row) =>
      row.id === "/rules/rust/file-size" ? { ...row, paths: ["**/*.rs", "build.rs"] } : row,
    );
    const view = repositoryView({ ...document, rows }, "owner/api", none);
    const terms = view?.selected?.fields.map((field) => field.term) ?? [];
    expect(terms.indexOf("Applies to")).toBe(terms.indexOf("Purpose") + 1);
    expect(view?.selected?.fields.find((field) => field.term === "Applies to")?.phrase).toEqual([
      { mono: "**/*.rs" },
      ", ",
      { mono: "build.rs" },
    ]);
    const without = repositoryView(fixtureDocument(), "owner/api", none);
    expect(without?.selected?.fields.map((field) => field.term)).not.toContain("Applies to");
  });

  it("names the newest run of a repository whose newest run uploaded none, and says so in its legend", () => {
    const view = repositoryView(withSupersededReport("owner/app"), "owner/app", none);
    expect(view?.headline).toBe("owner/app sent no report");
    expect(view?.subtitle).toBe("Main run #12 finished without uploading a controls report.");
    expect(view?.legend[3]?.meaning).toBe(
      "The repository's newest main run uploaded no report, so its rules have no state.",
    );
  });
});

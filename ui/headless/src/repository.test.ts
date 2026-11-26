import { describe, expect, it } from "vitest";
import { repositoryAddress, repositoryView } from "./repository.ts";
import { fixtureDocument, withStaleReport } from "./testDocuments.ts";

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

  it("lists the measured rules with their long checker and what they measured", () => {
    expect(
      view?.measured.rows.map((row) => [row.name, row.checker, row.measured, row.word]),
    ).toEqual([
      [
        "Keep each Rust file under 60 bytes",
        "budget · bytes per file · max 60",
        "1 file over: /src/main.rs 67",
        "failing",
      ],
      [
        "Pin Rust 1.99 or later",
        "version · at least 1.99",
        "/rust-toolchain.toml pins 1.90.0",
        "failing",
      ],
    ]);
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

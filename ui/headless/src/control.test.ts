import { describe, expect, it } from "vitest";
import { controlView } from "./control.ts";
import { fixtureDocument } from "./testDocuments.ts";

describe("controlView of one rule (artboard 57)", () => {
  const view = controlView(fixtureDocument(), "/rules/toolchains/rust");

  it("says in how many of the repositories it applies to it fails", () => {
    expect(view?.headline).toBe("rust fails in 1 of 3 repositories");
    expect(view?.breadcrumb).toEqual({ id: "/rules/groups/rust", title: "Rust" });
    expect(view?.source).toBe("imposed by controls · in groups Rust");
  });

  it("lists each repository it applies to, failing first, with its pin, run and drift", () => {
    expect(
      view?.repositories.map((line) => [
        line.name,
        line.word,
        line.measured,
        line.run,
        line.age,
        line.drift,
      ]),
    ).toEqual([
      ["owner/api", "failing", "/rust-toolchain.toml pins 1.90.0", null, "1 h", "drifts"],
      ["owner/app", "current", "/rust-toolchain.toml pins at least 1.99", null, "1 h", "drifts"],
      [
        "owner/ci",
        "current",
        "/rust-toolchain.toml pins at least 1.99",
        "run #7",
        "59 min",
        "drifts",
      ],
    ]);
  });

  it("says which repositories drift only in other rules", () => {
    expect(view?.notes).toEqual([
      "owner/app drifts in another rule: it overrides /rules/rust/server: requirement and lacks /rules/go/vet. It keeps this one as the set says.",
      "owner/api drifts in another rule: it lacks /rules/go/vet. It keeps this one as the set says.",
      "owner/ci drifts in another rule: it lacks /rules/go/vet. It keeps this one as the set says.",
    ]);
  });

  it("names the repositories it does not apply to, apart from those with no report", () => {
    expect(view?.notApplied).toEqual({
      names: [],
      toggle: "Show the 0 repositories it does not apply to",
    });
    expect(view?.noReport).toEqual(["owner/gone"]);
    const handlers = controlView(fixtureDocument(), "/rules/app/handlers");
    expect(handlers?.notApplied.names).toEqual(["owner/api", "owner/ci"]);
    expect(handlers?.notApplied.toggle).toBe("Show the 2 repositories it does not apply to");
  });

  it("says why the first failing repository fails and how to check it again", () => {
    expect(view?.failing).toEqual({
      repository: "owner/api",
      word: "failing",
      detail: "/rust-toolchain.toml line 1 pins 1.90.0, and the rule needs at least 1.99.",
      recheck: {
        argv: ["board", "rules", "report", "--path", "/repos/api"],
        hint: "Run it in a checkout of api, or re-run the controls job on main. The next read shows the new report.",
      },
    });
  });

  it("says what it checks, with each field of the rule", () => {
    expect(view?.checks.map((field) => [field.term, field.value])).toEqual([
      ["Requirement", "Pin Rust 1.99 or later."],
      ["Purpose", "Every repository builds on one toolchain."],
      ["Checker", "version · at least 1.99"],
      ["Reads", "/rust-toolchain.toml"],
      ["Capture", 'channel = "([0-9.]+)"'],
      ["Kind", "standing"],
      ["Shown", "failing: an agent sees it only while it fails"],
      ["Source", "Imposed. Named by controls in owner/controls."],
    ]);
  });

  it("rolls up each group it belongs to", () => {
    expect(
      view?.groups.map((group) => [group.title, group.controls, group.rollUp, group.word]),
    ).toEqual([
      ["Rust", "3 controls", "1 failing · 1 drifting · 1 no report · 1 current", "failing"],
    ]);
  });

  it("returns null for a rule the document does not hold", () => {
    expect(controlView(fixtureDocument(), "/rules/nowhere")).toBeNull();
  });
});

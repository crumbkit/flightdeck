import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Cost } from "@/api/model";
import { CostCard } from "./CostCard";

/** A command's cost with every figure that the tool reports today. */
const everyFigure: Cost = {
  figures: {
    runs: 86,
    help_pages: 19,
    help_before: 11,
    refused: 0,
    failed: 31,
    p95_ms: 65_100,
    p95_out_bytes: 1384,
  },
  findings: [
    {
      finding: "slow command",
      runs: 9,
      suggestion: "the 95th percentile of its wall time is 65,100 ms",
      code: null,
    },
    {
      finding: "retry after failure",
      runs: 11,
      suggestion: "process::exited failed and the command ran again within 60 s",
      code: "process::exited",
    },
    {
      finding: "retry after failure",
      runs: 7,
      suggestion: "workspace::not_current failed and the command ran again within 60 s",
      code: "workspace::not_current",
    },
    {
      finding: "identical repeat",
      runs: 18,
      suggestion: "the same call ran again within 60 s",
      code: null,
    },
  ],
};

/** Returns the tile titled `title`. */
function tile(title: string): HTMLElement {
  const heading = screen.getByRole("heading", { name: title });
  const found = heading.parentElement;
  if (found === null) {
    throw new Error(`${title} has no tile`);
  }
  return found;
}

/** Returns each term of a tile's rows with its count, as the page reads them. */
function rows(element: HTMLElement): string[][] {
  const terms = within(element).queryAllByRole("term");
  const counts = within(element).queryAllByRole("definition");
  return terms.map((term, index) => [term.textContent ?? "", counts[index]?.textContent ?? ""]);
}

describe("CostCard", () => {
  it("shows every figure the tool reports, the codes retried and the findings", () => {
    render(<CostCard cost={everyFigure} since="2026-10-27" />);

    const card = screen.getByRole("region", { name: "Cost to an agent" });
    expect(card).toHaveTextContent("86 runs since 27 Oct");
    expect(tile("Wall time")).toHaveTextContent("65.1 s95th percentile");
    expect(tile("Output an agent reads")).toHaveTextContent("1,384 B95th, about 346 tokens");
    expect(tile("Failed")).toHaveTextContent("31 of 8636% of runs");
    expect(rows(tile("Failed"))).toEqual([
      ["process::exited", "11"],
      ["workspace::not_current", "7"],
    ]);
    expect(rows(tile("Calls an agent could have saved"))).toEqual([
      ["Ran again after a failure", "18"],
      ["Same call again within 60 s", "18"],
      ["Read help before running", "11"],
      ["Help pages read", "19"],
      ["Usage errors", "0"],
    ]);
    const findings = within(screen.getByRole("list", { name: "Findings" })).getAllByRole(
      "listitem",
    );
    expect(findings.map((each) => each.textContent)).toEqual([
      "slowthe 95th percentile of its wall time is 65,100 ms (9 runs)",
      "retryprocess::exited failed and the command ran again within 60 s (11 runs)",
      "retryworkspace::not_current failed and the command ran again within 60 s (7 runs)",
      "identical repeatthe same call ran again within 60 s (18 runs)",
    ]);
  });

  it("leaves out each figure that the tool does not report", () => {
    const older: Cost = {
      figures: {
        runs: 4,
        help_pages: 2,
        help_before: null,
        refused: null,
        failed: null,
        p95_ms: null,
        p95_out_bytes: null,
      },
      findings: null,
    };

    render(<CostCard cost={older} since={null} />);

    const card = screen.getByRole("region", { name: "Cost to an agent" });
    expect(card).toHaveTextContent(/^Cost to an agent4 runs/);
    expect(
      within(card)
        .getAllByRole("heading")
        .map((each) => each.textContent),
    ).toEqual(["Cost to an agent", "Calls an agent could have saved"]);
    expect(rows(tile("Calls an agent could have saved"))).toEqual([["Help pages read", "2"]]);
    expect(screen.queryByRole("list", { name: "Findings" })).toBeNull();
  });

  it("shows the figures without a findings row or retried codes when there are no findings", () => {
    render(<CostCard cost={{ ...everyFigure, findings: [] }} since="2026-11-03" />);

    expect(screen.queryByRole("list", { name: "Findings" })).toBeNull();
    expect(rows(tile("Failed"))).toEqual([]);
    expect(rows(tile("Calls an agent could have saved"))).toEqual([
      ["Ran again after a failure", "0"],
      ["Same call again within 60 s", "0"],
      ["Read help before running", "11"],
      ["Help pages read", "19"],
      ["Usage errors", "0"],
    ]);
  });

  it("gathers the retried codes after the third under one row of other codes", () => {
    const retries = ["a::one", "b::two", "c::three", "d::four", "e::five"].map((code, index) => ({
      finding: "retry after failure",
      runs: 10 - index,
      suggestion: "",
      code,
    }));

    render(<CostCard cost={{ ...everyFigure, findings: retries }} since={null} />);

    expect(rows(tile("Failed"))).toEqual([
      ["a::one", "10"],
      ["b::two", "9"],
      ["c::three", "8"],
      ["2 other codes", "13"],
    ]);
  });
});

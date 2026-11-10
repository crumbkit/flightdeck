import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Cost, Finding } from "@/api/model";
import { CostCard } from "./CostCard";

/** Returns a finding of `kind` that counts `runs`, with `code` when given. */
function finding(kind: Finding["kind"], runs: number, code: string | null = null): Finding {
  return { kind, runs, code, p95_ms: null, p95_out_bytes: null };
}

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
    finding("identical_repeat", 18),
    finding("retry_after_failure", 7, "workspace::not_current"),
    finding("retry_after_failure", 11, "process::exited"),
    finding("help_loop", 9),
    { ...finding("slow_command", 9), p95_ms: 65_100 },
    finding("usage_error", 2),
  ],
};

const since = { year: 2026, month: 10, day: 27 };
const today = { year: 2026, month: 11, day: 10 };

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

/** Returns the text of each item of the findings strip. */
function strip(): string[] {
  const list = screen.getByRole("list", { name: "Findings" });
  return within(list)
    .getAllByRole("listitem")
    .map((each) => each.textContent ?? "");
}

describe("CostCard", () => {
  it("shows every figure the tool reports and phrases the findings that the tiles do not count", () => {
    render(<CostCard cost={everyFigure} command="up" since={since} today={today} />);

    const card = screen.getByRole("region", { name: "Cost to an agent" });
    expect(card).toHaveTextContent("86 runs, 27 Oct to 10 Nov");
    expect(tile("Wall time")).toHaveTextContent("65.1 s95th percentile");
    expect(tile("Output an agent reads")).toHaveTextContent("1,384 B95th, about 346 tokens");
    expect(tile("Failed")).toHaveTextContent(/^Failed31 of 8636% of runs$/);
    expect(rows(tile("Calls an agent could have saved"))).toEqual([
      ["Ran again after a failure", "18"],
      ["Same call again within 60 s", "18"],
      ["Read help before running", "11"],
      ["Help pages read", "19"],
      ["Usage errors", "0"],
    ]);
    expect(strip()).toEqual([
      "slowThe 95th percentile wall time is 65.1 s.",
      "help loopAgents read 3 or more help pages before up first succeeded, 9 pages in all.",
      "retryAfter process::exited, agents ran up again 11 times within 60 s.",
    ]);
    const list = screen.getByRole("list", { name: "Findings" });
    const mono = [...list.querySelectorAll("code")].map((each) => each.textContent);
    expect(mono).toEqual(["up", "process::exited", "up"]);
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

    render(<CostCard cost={older} command="up" since={null} today={today} />);

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

  it("shows the figures without a findings strip when no finding is one the card phrases", () => {
    const quiet = { ...everyFigure, findings: [finding("identical_repeat", 3)] };

    render(<CostCard cost={quiet} command="up" since={since} today={today} />);

    expect(screen.queryByRole("list", { name: "Findings" })).toBeNull();
    expect(rows(tile("Calls an agent could have saved"))[1]).toEqual([
      "Same call again within 60 s",
      "3",
    ]);
  });

  it("phrases a slow command from the card's figures and a retry without a code", () => {
    const findings = [finding("slow_command", 2), finding("retry_after_failure", 1)];

    render(
      <CostCard cost={{ ...everyFigure, findings }} command="up" since={since} today={today} />,
    );

    expect(strip()).toEqual([
      "slowThe 95th percentile wall time is 65.1 s.",
      "retryAfter a failure, agents ran up again 1 time within 60 s.",
    ]);
  });
});

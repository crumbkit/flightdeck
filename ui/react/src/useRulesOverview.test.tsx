import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { fixtureDocument, useMemoryAddress } from "./testing.tsx";
import { CHECKER_CHIPS, useRulesOverview } from "./useRulesOverview.ts";

const compliance = fixtureDocument();

function Page() {
  const address = useMemoryAddress();
  const page = useRulesOverview(compliance, address, { initialRows: 2 });
  return (
    <div>
      <h1>{page.view.headline}</h1>
      <fieldset>
        <legend>Checker</legend>
        {CHECKER_CHIPS.map(({ option, label }) => (
          <button key={option} {...page.getCheckerProps(option)}>
            {label}
          </button>
        ))}
      </fieldset>
      <table>
        <thead>
          <tr>
            <th {...page.getHeaderProps("rule")}>
              <button {...page.getSortButtonProps("rule")}>Rule</button>
            </th>
            <th {...page.getHeaderProps("state")}>
              <button {...page.getSortButtonProps("state")}>State</button>
            </th>
          </tr>
        </thead>
        <tbody>
          {page.rules.map((rule, index) => (
            <tr key={rule.key} {...page.getRuleRowProps(index)}>
              <td>{rule.name}</td>
              <td>{rule.word}</td>
            </tr>
          ))}
        </tbody>
      </table>
      {page.hidden > 0 ? <button {...page.getMoreButtonProps()}>{page.moreLabel}</button> : null}
      <output>{address.search}</output>
    </div>
  );
}

describe("useRulesOverview", () => {
  it("marks the sorted column with aria-sort, and sorts by another when its button is pressed", async () => {
    render(<Page />);
    const [rule, state] = screen.getAllByRole("columnheader");
    expect(state).toHaveAttribute("aria-sort", "ascending");
    expect(rule).not.toHaveAttribute("aria-sort");
    await userEvent.click(screen.getByRole("button", { name: "Rule" }));
    expect(screen.getByRole("status")).toHaveTextContent("?sort=rule");
    expect(rule).toHaveAttribute("aria-sort", "ascending");
    await userEvent.click(screen.getByRole("button", { name: "Rule" }));
    expect(rule).toHaveAttribute("aria-sort", "descending");
  });

  it("presses the chosen checker and keeps only its rules", async () => {
    render(<Page />);
    const chips = within(screen.getByRole("group", { name: "Checker" }));
    expect(chips.getByRole("button", { name: "All" })).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(chips.getByRole("button", { name: "written" }));
    expect(chips.getByRole("button", { name: "written" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("status")).toHaveTextContent("?checker=written");
    expect(screen.getAllByRole("row")).toHaveLength(3);
  });

  it("shows the first rules, and moves focus to the first one the button reveals", async () => {
    render(<Page />);
    const more = screen.getByRole("button", { name: "Show 4 more rules" });
    expect(more).toHaveAttribute("aria-expanded", "false");
    expect(screen.getAllByRole("row")).toHaveLength(3);
    await userEvent.click(more);
    const rows = screen.getAllByRole("row");
    expect(rows).toHaveLength(7);
    expect(rows[3]).toHaveFocus();
  });
});

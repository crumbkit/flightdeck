import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { fixtureDocument } from "./testing.tsx";
import { useControl } from "./useControl.ts";

const compliance = fixtureDocument();

function Page({ rule }: { rule: string }) {
  const page = useControl(compliance, rule);
  if (page.view === null) return <p>No such rule</p>;
  return (
    <div>
      <h1>{page.view.headline}</h1>
      <button {...page.getNotAppliedToggleProps()}>{page.view.notApplied.toggle}</button>
      <ul {...page.getNotAppliedRegionProps()}>
        {page.view.notApplied.names.map((name) => (
          <li key={name}>{name}</li>
        ))}
      </ul>
    </div>
  );
}

describe("useControl", () => {
  it("discloses the repositories the control does not apply to", async () => {
    render(<Page rule="/rules/app/handlers" />);
    const toggle = screen.getByRole("button", {
      name: "Show the 2 repositories it does not apply to",
    });
    expect(toggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("list")).toBeNull();
    await userEvent.click(toggle);
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("list")).toHaveAttribute("id", toggle.getAttribute("aria-controls"));
    expect(screen.getAllByRole("listitem").map((item) => item.textContent)).toEqual([
      "owner/api",
      "owner/ci",
    ]);
  });

  it("gives a null view for a rule the document does not hold", () => {
    render(<Page rule="/rules/nowhere" />);
    expect(screen.getByText("No such rule")).toBeInTheDocument();
  });
});

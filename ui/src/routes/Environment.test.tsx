import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { environmentShelf } from "../../tests/fixtures/shelf";
import Environment from "./Environment";

const variables = environmentShelf.environment ?? [];

/** Returns the table's row whose name button is `name`. */
function row(name: string): HTMLElement {
  const table = screen.getByRole("table", { name: "Environment variables", hidden: true });
  const button = within(table).getByRole("button", { name, hidden: true });
  const found = button.closest("tr");
  if (found === null) {
    throw new Error(`no row for ${name}`);
  }
  return found;
}

describe("Environment", () => {
  it("says how many variables there are, crumbkit's included, and what stops the tool", () => {
    render(<Environment name="shelf" variables={variables} open={null} />);

    expect(screen.getByText("16 declared · 1 required unset · 1 invalid")).toBeVisible();
    expect(screen.getByText("Values are as they were when flightdeck started.")).toBeVisible();
    expect(screen.getByText("shelf self environment list --full")).toBeVisible();
  });

  it("puts crumbkit's variables under their own heading after the tool's own", () => {
    render(<Environment name="shelf" variables={variables} open={null} />);
    const table = screen.getByRole("table", { name: "Environment variables", hidden: true });
    const headings = within(table).getAllByRole("columnheader", { hidden: true });
    const names = within(table)
      .getAllByRole("button", { hidden: true })
      .map((each) => each.textContent);

    expect(headings.map((each) => each.textContent)).toContain("Read by crumbkit in every tool");
    expect(names.indexOf("SHELF_REGION")).toBeLessThan(names.indexOf("SHELF_OUTPUT_BUDGET"));
  });

  it("draws a withheld value as eight dots and an unset one as not set", () => {
    render(<Environment name="shelf" variables={variables} open={null} />);

    const token = row("SHELF_MIRROR_TOKEN");
    expect(within(token).getByRole("img", { name: "withheld", hidden: true })).toHaveTextContent(
      "••••••••",
    );
    expect(within(row("SHELF_CATALOGUE_URL")).getByText("not set")).toBeInTheDocument();
  });

  it("says under a required unset row and an invalid row what is wrong", () => {
    render(<Environment name="shelf" variables={variables} open={null} />);
    const table = screen.getByRole("table", { name: "Environment variables", hidden: true });

    expect(
      within(table).getByText("Every books command fails, because shelf has no catalogue to read."),
    ).toBeInTheDocument();
    expect(
      within(table).getByText("The region must have a mirror that answers."),
    ).toBeInTheDocument();
  });

  it("opens a row to its purpose, effect, values, check, group and bind", async () => {
    const user = userEvent.setup();
    render(<Environment name="shelf" variables={variables} open={null} />);
    const table = screen.getByRole("table", { name: "Environment variables", hidden: true });

    await user.click(within(table).getByRole("button", { name: "SHELF_LOG_LEVEL", hidden: true }));
    await user.click(
      within(table).getByRole("button", { name: "SHELF_MIRROR_TOKEN", hidden: true }),
    );

    for (const text of [
      "The least severe message that shelf writes to its log.",
      "shelf logs at info.",
      "Each command that runs, and what it changed.",
      "mirror",
      "op://Personal/shelf mirror/token",
    ]) {
      expect(within(table).getAllByText(text).length).toBeGreaterThan(0);
    }
  });

  it("opens on the row of the variable it names", () => {
    render(<Environment name="shelf" variables={variables} open="SHELF_CATALOGUE_URL" />);
    const table = screen.getByRole("table", { name: "Environment variables", hidden: true });

    expect(
      within(table).getByRole("button", { name: "SHELF_CATALOGUE_URL", hidden: true }),
    ).toHaveAttribute("aria-expanded", "true");
    expect(
      within(table).getByText("The base URL of the catalogue that shelf reads."),
    ).toBeInTheDocument();
  });

  it("names each type in a word, the unset rule and the source as the mockups do", () => {
    render(<Environment name="shelf" variables={variables} open={null} />);
    const cells = (name: string) =>
      within(row(name))
        .getAllByRole("cell", { hidden: true })
        .map((each) => each.textContent);

    expect(cells("SHELF_PORT").slice(0, 3)).toEqual(["port", "default 8080", "setting"]);
    expect(cells("SHELF_CATALOGUE_URL").slice(0, 3)).toEqual(["url", "required", "setting"]);
    expect(cells("SHELF_LOG_LEVEL").slice(0, 3)).toEqual([
      "enumeration",
      "default info",
      "setting",
    ]);
    expect(cells("SHELF_MIRROR_TOKEN").slice(0, 3)).toEqual([
      "string",
      "optional locally, required in production",
      "secret · 1Password",
    ]);
    expect(
      within(row("SHELF_PORT")).getByText("8080", { selector: ".font-mono" }),
    ).toBeInTheDocument();
  });

  it("shows the text of a row from a tool that prints no structured fields", () => {
    const older = variables.map((each) => ({
      ...each,
      type_name: null,
      unset_rule: null,
      source_kind: null,
    }));
    render(<Environment name="shelf" variables={older} open={null} />);
    const cells = within(row("SHELF_LOG_LEVEL"))
      .getAllByRole("cell", { hidden: true })
      .map((each) => each.textContent);

    expect(cells.slice(0, 3)).toEqual([
      "log-level: one of error, warn, info, debug",
      "default info",
      "setting",
    ]);
  });

  it("says a tool that declares nothing declares no variables", () => {
    render(<Environment name="shelf" variables={[]} open={null} />);

    expect(screen.getByText("shelf declares no environment variables.")).toBeVisible();
  });
});

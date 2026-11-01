import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { shelfWithoutGoals } from "../../tests/fixtures/shelf";
import { Rail } from "./Rail";
import { Welcome } from "./Welcome";

/** The command paths of the tool, such as "books list", in model order. */
const paths = shelfWithoutGoals.commands.map((each) => each.path.join(" "));

describe("a tool without goal headings", () => {
  it("has a rail with no heading and a button for every command", () => {
    render(<Rail tool={shelfWithoutGoals} selected={null} onChoose={() => {}} />);
    const rail = screen.getByRole("navigation", { name: "Commands" });

    expect(within(rail).queryAllByRole("heading")).toHaveLength(0);
    const buttons = within(rail).getAllByRole("button");
    expect(buttons).toHaveLength(paths.length);
    for (const [index, path] of paths.entries()) {
      expect(buttons[index]).toHaveTextContent(path);
    }
  });

  it("has a welcome card with no goal heading and every command", () => {
    render(<Welcome tool={shelfWithoutGoals} onExample={() => {}} />);
    const welcome = screen.getByRole("region", { name: "Welcome to shelf" });

    expect(within(welcome).queryAllByRole("heading", { level: 2 })).toHaveLength(0);
    for (const path of paths) {
      expect(welcome).toHaveTextContent(path);
    }
  });
});

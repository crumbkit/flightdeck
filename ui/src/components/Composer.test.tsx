import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Command } from "@/api/model";
import { emptyDraft } from "@/lib/form";
import { command, shelf } from "../../tests/fixtures/shelf";
import { Composer } from "./Composer";

const noop = () => undefined;

function composer(shown: Command, focus: number | null) {
  return (
    <Composer
      command={shown}
      draft={emptyDraft(shown)}
      editing={focus === null ? null : 1}
      focus={focus}
      readOnly={false}
      fieldError={null}
      lineRefused={false}
      onChange={noop}
      onRun={noop}
      onStopEditing={noop}
      onShowRun={noop}
    />
  );
}

describe("Composer", () => {
  it("moves focus to the first field again on a second Edit of the same run", () => {
    const show = command(shelf, "books show");
    const { rerender } = render(composer(show, 1));
    const field = screen.getByLabelText(/The ids of the books to show/);
    expect(field).toHaveFocus();

    field.blur();
    rerender(composer(show, 2));

    expect(field).toHaveFocus();
  });

  it("focuses a checkbox that is the first field, and opens More options to reach it", () => {
    const list = command(shelf, "books list");
    const onlyAll = {
      ...list,
      arguments: list.arguments.filter((argument) => argument.name === "--all"),
    };

    render(composer(onlyAll, 1));

    expect(screen.getByRole("checkbox")).toHaveFocus();
    expect(screen.getByText("More options").closest("details")).toHaveAttribute("open");
  });
});

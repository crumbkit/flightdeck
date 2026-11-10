import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DirectoryControl } from "./DirectoryControl";

const change = { name: "Working directory /home/you/notes. Change" };

function renderControl(directory: string | null = "/home/you/notes") {
  const onChange = vi.fn<(directory: string) => void>();
  render(<DirectoryControl directory={directory} onChange={onChange} />);
  return onChange;
}

describe("DirectoryControl", () => {
  it("shows the working directory on a button that says it can be changed", () => {
    renderControl();

    expect(screen.getByRole("button", change)).toHaveTextContent("/home/you/notes");
  });

  it("takes a typed directory on Enter and gives focus back to its button", async () => {
    const onChange = renderControl();

    await userEvent.click(screen.getByRole("button", change));
    const box = screen.getByRole("textbox", { name: "Working directory" });
    expect(box).toHaveFocus();
    expect(box).toHaveValue("/home/you/notes");
    await userEvent.clear(box);
    await userEvent.type(box, "/home/you/books{Enter}");

    expect(onChange).toHaveBeenCalledExactlyOnceWith("/home/you/books");
    expect(screen.getByRole("button", change)).toHaveFocus();
  });

  it("takes a typed directory when focus leaves the box, so a press of Run uses it", async () => {
    const onChange = renderControl();

    await userEvent.click(screen.getByRole("button", change));
    const box = screen.getByRole("textbox", { name: "Working directory" });
    await userEvent.clear(box);
    await userEvent.type(box, "/home/you/books");
    await userEvent.tab();

    expect(onChange).toHaveBeenCalledExactlyOnceWith("/home/you/books");
  });

  it("keeps the directory when Escape closes the box", async () => {
    const onChange = renderControl();

    await userEvent.click(screen.getByRole("button", change));
    await userEvent.type(
      screen.getByRole("textbox", { name: "Working directory" }),
      "/more{Escape}",
    );

    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole("button", change)).toHaveFocus();
  });

  it("keeps the directory when the box is left blank", async () => {
    const onChange = renderControl();

    await userEvent.click(screen.getByRole("button", change));
    await userEvent.clear(screen.getByRole("textbox", { name: "Working directory" }));
    await userEvent.keyboard("{Enter}");

    expect(onChange).not.toHaveBeenCalled();
  });

  it("says that runs start where flightdeck started when the server named no directory", () => {
    renderControl(null);

    expect(
      screen.getByRole("button", { name: "Working directory where flightdeck started. Change" }),
    ).toHaveTextContent("where flightdeck started");
  });
});

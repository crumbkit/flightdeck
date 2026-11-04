import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { makeQueryClient } from "@/queryClient";
import { ToolPage } from "@/routes/ToolPage";
import { shelfReply } from "../../tests/fixtures/shelf";

/** The bodies of the runs the page asked the server for. */
let runs: unknown[] = [];

beforeEach(() => {
  runs = [];
  vi.stubGlobal("fetch", async (path: string, init: RequestInit) => {
    runs.push({ path, body: JSON.parse(typeof init.body === "string" ? init.body : "null") });
    const lines = [
      { event: "started", run: "r1", line: "shelf", started_at: "2026-11-02T09:00:00Z" },
      { event: "ended", outcome: { kind: "result" }, exit: 0, duration_ms: 1 },
    ];
    return new Response(lines.map((line) => `${JSON.stringify(line)}\n`).join(""));
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

function renderPage() {
  render(
    <QueryClientProvider client={makeQueryClient()}>
      <ToolPage reply={shelfReply} onSignedOut={() => undefined} />
    </QueryClientProvider>,
  );
  return { user: userEvent.setup(), line: screen.getByRole("textbox", { name: /Command line/ }) };
}

function input(label: RegExp | string): HTMLInputElement {
  const found = screen.getByLabelText(label);
  if (!(found instanceof HTMLInputElement)) {
    throw new Error(`${String(label)} is not an input`);
  }
  return found;
}

describe("LineEditor", () => {
  it("changes no field while an option is typed, and fills it once a space follows its value", async () => {
    const { user, line } = renderPage();

    await user.type(line, "books list --li");
    const limit = input(/Print at most this many rows/);
    expect(limit).toHaveValue("");

    await user.type(line, "mit 1");
    expect(limit).toHaveValue("");
    await user.type(line, " ");
    expect(limit).toHaveValue("1");
  });

  it("keeps the text and the caret as typed while the line has focus, and rewrites it in the form's order on leaving", async () => {
    const { user, line } = renderPage();

    await user.type(line, "books list --all --limit=10 ");
    expect(input(/Print at most this many rows/)).toHaveValue("10");
    expect(line).toHaveValue("books list --all --limit=10 ");
    expect(line).toHaveProperty("selectionStart", 28);

    await user.type(line, "x", { initialSelectionStart: 10, initialSelectionEnd: 10 });
    expect(line).toHaveValue("books listx --all --limit=10 ");
    expect(line).toHaveProperty("selectionStart", 11);

    await user.keyboard("{Backspace}");
    await user.tab();
    expect(line).toHaveValue("books list --limit 10 --all");
  });

  it("rewrites the line while a field changes", async () => {
    const { user, line } = renderPage();
    await user.type(line, "books list");
    await user.tab();

    await user.click(input(/Print every row/));

    expect(line).toHaveValue("books list --all");
  });

  it("underlines an unknown option, keeps Run off and fixes the option with one click", async () => {
    const { user, line } = renderPage();

    await user.type(line, "books list --limti 10");
    await user.tab();

    expect(line).toHaveAttribute("aria-invalid", "true");
    expect(line).toHaveAccessibleDescription(
      "unexpected argument '--limti' found; a similar argument exists: '--limit'",
    );
    expect(screen.getByRole("button", { name: /^Run/ })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: "Change to --limit" }));
    expect(line).toHaveValue("books list --limit 10");
    expect(input(/Print at most this many rows/)).toHaveValue("10");
    expect(screen.getByRole("button", { name: /^Run/ })).toBeEnabled();
  });

  it("fills the ids and the long columns from a line pasted from a terminal", async () => {
    const { user, line } = renderPage();

    await user.click(line);
    await user.paste("$ shelf books show 2 5 --long");
    await user.tab();

    const composer = screen.getByRole("form", { name: "Composer" });
    expect(within(composer).getByRole("button", { name: "Remove 2" })).toBeInTheDocument();
    expect(within(composer).getByRole("button", { name: "Remove 5" })).toBeInTheDocument();
    expect(within(composer).getByRole("checkbox", { name: /Long columns/ })).toBeChecked();
    expect(line).toHaveValue("books show 2 5 --long");
  });

  it("takes --yes out of a pasted line, says why, and runs nothing", async () => {
    const { user, line } = renderPage();

    await user.click(line);
    await user.paste("shelf books remove 2 --yes");
    await user.keyboard("{Enter}");

    expect(line).toHaveValue("books remove 2");
    expect(screen.getByText(/so --yes was taken out of the line/)).toBeInTheDocument();
    expect(runs).toEqual([]);
  });

  it("runs the line on Enter with structured values", async () => {
    const { user, line } = renderPage();

    await user.type(line, "books show 2 5{Enter}");

    expect(await screen.findByRole("region", { name: /^Run 1:/ })).toBeInTheDocument();
    expect(runs).toEqual([
      { path: "/api/runs", body: { path: ["books", "show"], values: { "<ID>": ["2", "5"] } } },
    ]);
  });
});

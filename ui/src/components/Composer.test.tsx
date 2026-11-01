import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ChoicesReply } from "@/api/choices";
import { type Draft, emptyDraft } from "@/lib/form";
import { makeQueryClient } from "@/queryClient";
import { command, shelf } from "../../tests/fixtures/shelf";
import { Composer } from "./Composer";

afterEach(() => {
  vi.unstubAllGlobals();
});

const noop = () => undefined;

/** Two of shelf's books, as `POST /api/choices` sends them. */
const twoBooks: ChoicesReply = {
  source: ["books", "list"],
  column: "id",
  labels: ["title", "author", "year"],
  choices: [
    { value: "1", label: ["Pride and Prejudice", "Jane Austen", 1813] },
    { value: "2", label: ["Frankenstein", "Mary Shelley", 1818] },
  ],
  truncated: false,
};

/** Stubs fetch with `reply` for the choices route, and returns the bodies it was sent. */
function stubChoices(reply: Response): string[] {
  const sent: string[] = [];
  vi.stubGlobal("fetch", async (path: string, init: RequestInit) => {
    expect(path).toBe("/api/choices");
    sent.push(typeof init.body === "string" ? init.body : "");
    return reply.clone();
  });
  return sent;
}

const booksShow = command(shelf, "books show");

/** The composer of books show, which keeps its own draft and reports each change. */
function BooksShow({ onDraft }: { onDraft: (draft: Draft) => void }) {
  const [draft, setDraft] = useState(emptyDraft(booksShow));
  return (
    <Composer
      tool={shelf}
      command={booksShow}
      draft={draft}
      editing={null}
      readOnly={false}
      fieldError={null}
      onChange={(next) => {
        setDraft(next);
        onDraft(next);
      }}
      onRun={noop}
      onStopEditing={noop}
      onShowRun={noop}
    />
  );
}

/** Renders the composer of books show, and returns its latest draft. */
function renderBooksShow(): { draft: () => Draft } {
  let latest = emptyDraft(booksShow);
  render(
    <QueryClientProvider client={makeQueryClient()}>
      <BooksShow
        onDraft={(draft) => {
          latest = draft;
        }}
      />
    </QueryClientProvider>,
  );
  return { draft: () => latest };
}

describe("Composer's IDs chooser", () => {
  it("asks for the choices of books show from books list and labels each by its other columns", async () => {
    const sent = stubChoices(Response.json(twoBooks));

    renderBooksShow();

    const choices = await screen.findByRole("group", { name: "Choices" });
    expect(choices).toHaveTextContent("Frankenstein · Mary Shelley · 1818");
    expect(sent).toEqual([
      JSON.stringify({ source: ["books", "list"], target: ["books", "show"], argument: "<ID>" }),
    ]);
    expect(screen.getByText("From shelf books list, column id · inferred")).toBeVisible();
  });

  it("adds a checked choice beside a typed 9, which is no choice", async () => {
    stubChoices(Response.json(twoBooks));
    const user = userEvent.setup();
    const { draft } = renderBooksShow();

    await user.type(screen.getByRole("textbox", { name: /<ID>/ }), "9 ");
    await user.click(await screen.findByRole("checkbox", { name: /^2 Frankenstein/ }));

    expect(draft().values["<ID>"]).toEqual(["9", "2"]);
    expect(screen.getByRole("button", { name: "Remove 9" })).toBeVisible();
    expect(screen.getByRole("checkbox", { name: /^2 Frankenstein/ })).toBeChecked();
  });

  it("says when the source has more rows than the chooser lists", async () => {
    stubChoices(Response.json({ ...twoBooks, truncated: true }));

    renderBooksShow();

    expect(
      await screen.findByText("Showing the first 2 choices. Type any other id."),
    ).toBeVisible();
  });

  it("asks for typed ids when the choices cannot be listed", async () => {
    stubChoices(
      Response.json(
        {
          code: "flightdeck::choices::time_limit",
          message: "books list did not list its choices in time",
        },
        { status: 504 },
      ),
    );

    renderBooksShow();

    expect(
      await screen.findByText("flightdeck could not list the choices. Type the ids instead."),
    ).toBeVisible();
    expect(screen.queryByRole("group", { name: "Choices" })).toBeNull();
  });
});

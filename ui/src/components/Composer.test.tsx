import { type QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ChoicesReply } from "@/api/choices";
import type { Command } from "@/api/model";
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
      focus={null}
      readOnly={false}
      fieldError={null}
      lineRefused={false}
      directory={null}
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

/** The composer of `shown` after an Edit that asks for focus `focus`. */
function edited(shown: Command, focus: number | null, client: QueryClient) {
  return (
    <QueryClientProvider client={client}>
      <Composer
        tool={shelf}
        command={shown}
        draft={emptyDraft(shown)}
        editing={focus === null ? null : 1}
        focus={focus}
        readOnly={false}
        fieldError={null}
        lineRefused={false}
        directory={null}
        onChange={noop}
        onRun={noop}
        onStopEditing={noop}
        onShowRun={noop}
      />
    </QueryClientProvider>
  );
}

describe("Composer", () => {
  it("moves focus to the first field again on a second Edit of the same run", () => {
    stubChoices(Response.json(twoBooks));
    const client = makeQueryClient();
    const { rerender } = render(edited(booksShow, 1, client));
    const field = screen.getByLabelText(/The ids of the books to show/);
    expect(field).toHaveFocus();

    field.blur();
    rerender(edited(booksShow, 2, client));

    expect(field).toHaveFocus();
  });

  it("focuses a checkbox that is the first field, and opens More options to reach it", () => {
    const list = command(shelf, "books list");
    const onlyAll = {
      ...list,
      arguments: list.arguments.filter((argument) => argument.name === "--all"),
    };

    render(edited(onlyAll, 1, makeQueryClient()));

    expect(screen.getByRole("checkbox")).toHaveFocus();
    expect(screen.getByText("More options").closest("details")).toHaveAttribute("open");
  });

  it("shows the command's cost between its about line and its fields, and no card without one", () => {
    const list = command(shelf, "books list");
    const costed: Command = {
      ...list,
      cost: {
        figures: {
          runs: 3,
          help_pages: 0,
          help_before: 1,
          refused: 0,
          failed: 0,
          p50_ms: 6,
          p95_ms: 12,
          p50_out_bytes: 235,
          p95_out_bytes: 648,
        },
        findings: [],
      },
    };

    const { rerender } = render(edited(costed, null, makeQueryClient()));

    const card = screen.getByRole("region", { name: "Cost to an agent" });
    const about = screen.getByText(list.about ?? "");
    const fields = screen.getByText("More options");
    expect(about.compareDocumentPosition(card)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
    expect(card.compareDocumentPosition(fields)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);

    rerender(edited(list, null, makeQueryClient()));

    expect(screen.queryByRole("region", { name: "Cost to an agent" })).toBeNull();
  });
});

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

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Command } from "@/api/model";
import type { RunRequest } from "@/api/runs";
import type { Run, RunState } from "@/lib/transcript";
import { command, shelf } from "../../tests/fixtures/shelf";
import { RunCard } from "./RunCard";

afterEach(() => {
  vi.restoreAllMocks();
});

const noop = () => undefined;
const startedAt = "2026-11-02T09:00:00Z";

/** Renders a card of `shown`, books show by default, in `state`, and returns what it composes. */
function renderCard(
  state: RunState,
  { line = "shelf books show 2 7 9", shown = command(shelf, "books show") } = {},
) {
  const composed: RunRequest[] = [];
  const run: Run = {
    number: 1,
    request: { path: shown.path, values: {} },
    line,
    long: false,
    state,
  };
  render(
    <RunCard
      run={run}
      tool={shelf}
      command={shown}
      link={null}
      cancelWords={null}
      onEdit={noop}
      onRunAgain={noop}
      onCancel={noop}
      onPageSize={noop}
      onCompose={(request) => composed.push(request)}
      onOpenLink={noop}
    />,
  );
  return composed;
}

function ended(lines: unknown[], exit = 0): RunState {
  return {
    kind: "ended",
    startedAt,
    lines: lines.map((line) => JSON.stringify(line)),
    exit,
    durationMs: 3,
    outcome: { kind: exit === 0 ? "result" : "result_with_failures" },
  };
}

const notFound = (key: string, index: number) => ({
  key,
  index,
  code: "shelf::books::not_found",
  message: `the catalogue has no book ${key}`,
  fields: { books: [Number(key)] },
  recovery: { kind: "run", call: "shelf books list" },
});

const list = command(shelf, "books list");

/** `books list` as if it streamed its rows, as `reading entries stream` does. */
const streaming: Command = { ...list, result: { ...list.result, form: "stream" } };

const item = (id: number) => JSON.stringify({ schema: "shelf/books-list/v1", data: { id } });

describe("RunCard", () => {
  it("lists the same item failure twice without a duplicate key", () => {
    const errors = vi.spyOn(console, "error").mockImplementation(noop);
    const failure = notFound("7", 0);

    renderCard(ended([{ schema: "s", data: [], failures: [failure, failure] }], 1));

    expect(screen.getAllByRole("alert")).toHaveLength(2);
    expect(errors).not.toHaveBeenCalled();
  });

  it("shows no code for a refusal that came without a document", () => {
    renderCard({ kind: "refused", problem: null, words: "The server refused the request." });

    expect(screen.getByRole("alert").querySelector("code")).toBeNull();
  });

  it("draws a batch's failures in key order, each headed by its key and index", () => {
    renderCard(
      ended(
        [
          {
            schema: "shelf/books-show/v1",
            data: [{ id: 2, title: "Frankenstein" }],
            failures: [notFound("9", 2), notFound("7", 1)],
          },
        ],
        1,
      ),
    );

    const cards = screen.getAllByRole("alert");
    expect(cards.map((card) => card.querySelector("p")?.textContent)).toEqual([
      "7 (index 1)the catalogue has no book 7",
      "9 (index 2)the catalogue has no book 9",
    ]);
    expect(within(cards[0] ?? document.body).getByText("shelf::books::not_found")).toBeVisible();
  });

  it("fills the composer from a failure's recovery chip and runs nothing", async () => {
    const user = userEvent.setup();
    const composed = renderCard(
      ended([{ schema: "s", data: [], failures: [notFound("7", 0)] }], 1),
    );

    await user.click(screen.getByRole("button", { name: "books list" }));

    expect(composed).toEqual([{ path: ["books", "list"], values: {} }]);
  });

  it("underlines the refused word of the line and draws the tool's alternatives as quiet chips", () => {
    const failure = {
      code: "shelf::cli::usage",
      message: "--limit takes a whole number from 1 to 100",
      fields: { argument: "--limit", value: "101", most: 100 },
      recovery: { kind: "run", call: "shelf books list --all --json" },
      alternatives: [{ kind: "run", call: "shelf books list --limit 100 --json" }],
    } as const;
    const state: RunState = {
      kind: "ended",
      startedAt,
      lines: [JSON.stringify({ schema: "shelf/failure/v1", ...failure })],
      exit: 2,
      durationMs: 3,
      outcome: {
        kind: "tool_failure",
        failure: { ...failure, alternatives: [...failure.alternatives] },
      },
    };

    renderCard(state, { line: "shelf books list --limit=101", shown: list });

    expect(screen.getByText("--limit=101")).toHaveClass("decoration-wavy");
    expect(screen.getByRole("button", { name: "books list --all --json" })).toHaveClass(
      "bg-primary",
    );
    expect(screen.getByRole("button", { name: "books list --limit 100 --json" })).toHaveClass(
      "bg-muted",
    );
  });

  it("shows a listing's next step as a chip after its footer", () => {
    const step = {
      text: "takes up to 100 ids in one call.",
      commands: [{ call: "shelf books show <ID>..." }],
    };
    const page = { schema: "s", data: [{ id: 1 }, { id: 2 }], total: 6, truncated: true };

    renderCard(ended([{ ...page, next: [step] }]), { line: "shelf books list", shown: list });

    expect(screen.getByRole("button", { name: "books show <ID>…" })).toBeVisible();
  });

  it("shows no next chip for a result without next steps", () => {
    const page = { schema: "s", data: [{ id: 1 }, { id: 2 }], total: 6, truncated: true };

    renderCard(ended([page]), { line: "shelf books list", shown: list });

    expect(screen.queryByText("Next:")).toBeNull();
  });

  it("shows three next chips and puts the rest behind +N more", async () => {
    const user = userEvent.setup();
    const calls = ["books show 1", "books show 2", "books show 3", "books show 4", "books show 5"];
    const steps = calls.map((call) => ({ text: "", commands: [{ call: `shelf ${call}` }] }));
    renderCard(ended([{ schema: "s", data: [{ id: 1 }, { id: 2 }], next: steps }]));

    expect(screen.queryByRole("button", { name: "books show 4" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "+2 more" }));

    expect(screen.getByRole("button", { name: "books show 5" })).toBeVisible();
  });

  it("draws a stream's rows as they arrive, with Cancel and a count of its rows", () => {
    renderCard(
      { kind: "running", id: "r1", startedAt, lines: [item(1), item(2)] },
      { line: "shelf books list", shown: streaming },
    );

    expect(screen.getAllByRole("row")).toHaveLength(3);
    expect(screen.getByRole("button", { name: "Cancel" })).toBeVisible();
    expect(
      screen.getByRole("region", { name: "Run 1: shelf books list, running · 2 rows" }),
    ).toBeVisible();
  });

  it("keeps a cancelled run's rows and says how many it kept", () => {
    renderCard(
      {
        kind: "ended",
        startedAt,
        lines: [item(1), item(2)],
        exit: 130,
        durationMs: 900,
        outcome: { kind: "cancelled" },
      },
      { line: "shelf books list", shown: streaming },
    );

    expect(
      screen.getByText("cancelled · exit 130 · 2 rows kept", { selector: "span.font-mono" }),
    ).toBeVisible();
    expect(screen.getAllByRole("row")).toHaveLength(3);
    expect(screen.getByText(/cancelled, so its result is partial/)).toBeVisible();
  });

  it("keeps the rows of a stream whose connection broke, and says the connection broke", () => {
    renderCard(
      {
        kind: "cut",
        startedAt,
        lines: [item(1)],
        words: "The connection to flightdeck broke before the run ended.",
      },
      { line: "shelf books list", shown: streaming },
    );

    expect(screen.getAllByRole("row")).toHaveLength(2);
    expect(screen.getByRole("alert")).toHaveTextContent(/connection to flightdeck broke/);
  });

  it("says a finished run's status with its rows in its live region", () => {
    const page = {
      schema: "s",
      data: [{ id: 1 }, { id: 2 }, { id: 3 }],
      total: 3,
      truncated: false,
    };

    renderCard(ended([page]), { line: "shelf books list", shown: list });

    expect(screen.getByRole("status")).toHaveTextContent("exit 0 · 3 rows");
  });
});

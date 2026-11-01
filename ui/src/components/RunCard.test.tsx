import { render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Run } from "@/lib/transcript";
import { command, shelf } from "../../tests/fixtures/shelf";
import { RunCard } from "./RunCard";

afterEach(() => {
  vi.restoreAllMocks();
});

const noop = () => undefined;

describe("RunCard", () => {
  it("lists the same item failure twice without a duplicate key", () => {
    const errors = vi.spyOn(console, "error").mockImplementation(noop);
    const failure = { key: "7", code: "shelf::books::not_found", message: "no book has id 7" };
    const run: Run = {
      number: 1,
      request: { path: ["books", "show"], values: { "<ID>": ["7", "7"] } },
      line: "shelf books show 7 7",
      state: {
        kind: "ended",
        startedAt: "2026-11-02T09:00:00Z",
        lines: [
          JSON.stringify({ schema: "shelf/books-show/v1", data: [], failures: [failure, failure] }),
        ],
        exit: 1,
        durationMs: 3,
        outcome: { kind: "result_with_failures" },
      },
    };

    const { getAllByRole } = render(
      <RunCard
        run={run}
        command={command(shelf, "books show")}
        link={null}
        onEdit={noop}
        onRunAgain={noop}
        onPageSize={noop}
        onOpenLink={noop}
      />,
    );

    expect(getAllByRole("alert")).toHaveLength(2);
    expect(errors).not.toHaveBeenCalled();
  });

  it("shows no code for a refusal that came without a document", () => {
    const run: Run = {
      number: 1,
      request: { path: ["books", "show"], values: {} },
      line: "shelf books show",
      state: { kind: "refused", problem: null, words: "The server refused the request." },
    };

    const { getByRole } = render(
      <RunCard
        run={run}
        command={command(shelf, "books show")}
        link={null}
        onEdit={noop}
        onRunAgain={noop}
        onPageSize={noop}
        onOpenLink={noop}
      />,
    );

    expect(getByRole("alert").querySelector("code")).toBeNull();
  });
});

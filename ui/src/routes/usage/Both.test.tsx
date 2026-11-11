import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Command, Tool } from "@/api/model";
import type { WeightReply } from "@/api/weight";
import { makeQueryClient } from "@/queryClient";
import { command as commandOf, olderShelf, shelf } from "../../../tests/fixtures/shelf";
import Both, { BothSummary } from "./Both";
import Weight from "./Weight";

/**
 * Returns the command at `path` of `from`, shelf by default, run `runs` times with `p95` bytes of
 * output.
 */
function ran(path: string, runs: number, p95: number | null = null, from: Tool = shelf): Command {
  const figures = {
    runs,
    help_pages: 0,
    help_before: 0,
    refused: 0,
    failed: 0,
    p50_ms: 1,
    p95_ms: 2,
    p50_out_bytes: 1,
    p95_out_bytes: p95,
  };
  const command = commandOf(from, path);
  return { ...command, runs: command.counted ? runs : 0, cost: { figures, findings: null } };
}

const tool: Tool = {
  ...shelf,
  figures_since: null,
  commands: [
    ran("books list", 90, 4000), // 5,500 B: heavy and often
    ran("books show", 50, 100), // 1,600 B: light and often
    ran("books summarize", 10, 3000), // 4,500 B: in no corner
    ran("books remove", 0), // 1,500 B: light and never
    ran("self usage", 900, 9000, olderShelf), // not counted on an earlier crumbkit
  ],
};

const page = (path: string, bytes: number | null) => ({
  path: path === "" ? [] : path.split(" "),
  bytes,
  failure: bytes === null ? ("timed_out" as const) : null,
  exit: null,
});

const measured: WeightReply = {
  pages: [
    "",
    "books",
    "books list",
    "books show",
    "books summarize",
    "books remove",
    "self",
    "self usage",
  ].map((path) => page(path, 500)),
  left_out: 0,
  problem: null,
};

/** Stubs `GET /api/weight` with `reply`, and returns how often the page asked. */
function stubWeight(reply: WeightReply): { asked: number } {
  const count = { asked: 0 };
  vi.stubGlobal("fetch", async () => {
    count.asked += 1;
    return Response.json(reply);
  });
  return count;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

function renderBoth(onOpen: (command: string) => void = () => {}) {
  render(
    <QueryClientProvider client={makeQueryClient()}>
      <span data-testid="summary">
        <BothSummary tool={tool} today={{ year: 2026, month: 11, day: 10 }} />
      </span>
      <Both tool={tool} onOpen={onOpen} />
    </QueryClientProvider>,
  );
}

/** Returns the table's rows as the text of their command and read cells. */
function rows(): string[][] {
  const table = screen.getByRole("table", { name: "Path weight and runs of each command" });
  return within(table)
    .getAllByRole("row")
    .slice(1)
    .map((row) => {
      const cells = [...row.querySelectorAll("td")];
      return [cells[0]?.textContent ?? "", cells.at(-1)?.textContent ?? ""];
    });
}

describe("Both", () => {
  it("lists every command under All, the most read first, with the not-counted one last", async () => {
    const asked = stubWeight(measured);

    renderBoth();

    await screen.findByRole("table", { name: "Path weight and runs of each command" });
    expect(rows()).toEqual([
      ["books list", "about 124k tokens"],
      ["books show", "about 20k tokens"],
      ["books summarize", "about 11k tokens"],
      ["books remove", "none"],
      ["self usage", "not counted"],
    ]);
    expect(screen.getByTestId("summary")).toHaveTextContent(
      "Path weight times runs · heavy is at least 4,000 B, half the default path budget · often is at least 50 runs, the median of commands that ran",
    );
    expect(asked.asked).toBe(1);
  });

  it("shows a partial measurement's rows under a line that offers to measure again", async () => {
    stubWeight({
      ...measured,
      pages: measured.pages.map((each) =>
        each.path.join(" ") === "books remove"
          ? { ...each, bytes: null, failure: "not_run" as const }
          : each,
      ),
    });

    renderBoth();

    await screen.findByRole("table", { name: "Path weight and runs of each command" });
    expect(screen.getByRole("status")).toHaveTextContent(
      "flightdeck stopped measuring before 1 help page, at its 30 s limit or its own stop.",
    );
    expect(screen.getByRole("button", { name: "Measure again" })).toBeInTheDocument();
  });

  it("shows each corner's count on its chip, and a chosen chip keeps its rows and names its action", async () => {
    stubWeight(measured);
    renderBoth();
    const chips = await screen.findByRole("group", { name: "Corners" });

    expect(
      within(chips)
        .getAllByRole("button")
        .map((chip) => chip.textContent),
    ).toEqual([
      "All 5",
      "Heavy and often 1",
      "Light and never 1",
      "Heavy and never 0",
      "Light and often 1",
    ]);
    expect(within(chips).getByRole("button", { name: /^All/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    await userEvent.click(within(chips).getByRole("button", { name: /^Light and often/ }));

    expect(within(chips).getByRole("button", { name: /^Light and often/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(rows()).toEqual([["books show", "about 20k tokens"]]);
    expect(screen.getByText(/^Light and often:/).parentElement).toHaveTextContent(
      "Light and often: nothing to do.",
    );
  });

  it("calls a command whose path has a page that was not measured at least its weight", async () => {
    stubWeight({
      ...measured,
      pages: measured.pages.map((each) =>
        each.path.join(" ") === "books" ? page("books", null) : each,
      ),
    });

    renderBoth();

    await screen.findByRole("table", { name: "Path weight and runs of each command" });
    expect(rows()[0]).toEqual(["books list", "at least 113k tokens"]);
  });

  it("opens a command's form from its name", async () => {
    stubWeight(measured);
    const opened: string[] = [];
    renderBoth((command) => opened.push(command));
    const table = await screen.findByRole("table", {
      name: "Path weight and runs of each command",
    });

    await userEvent.click(within(table).getByRole("button", { name: "books show" }));

    expect(opened).toEqual(["books show"]);
  });

  it("says why when flightdeck could not measure the help pages", async () => {
    stubWeight({ pages: [], left_out: 0, problem: "cannot make a temporary state directory" });

    renderBoth();

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "flightdeck could not measure help pages: cannot make a temporary state directory",
    );
  });

  it("reads the Weight tab's measurement, so the two tabs ask for the help pages once", async () => {
    const asked = stubWeight(measured);

    render(
      <QueryClientProvider client={makeQueryClient()}>
        <Weight tool={tool} />
        <Both tool={tool} onOpen={() => {}} />
      </QueryClientProvider>,
    );

    await screen.findByRole("table", { name: "Path weight and runs of each command" });
    await screen.findByRole("table", { name: "Weight of each command's path" });
    expect(asked.asked).toBe(1);
  });
});

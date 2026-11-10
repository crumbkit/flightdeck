import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Command, Tool } from "@/api/model";
import type { WeightReply } from "@/api/weight";
import { makeQueryClient } from "@/queryClient";
import { command as commandOf, shelf } from "../../../tests/fixtures/shelf";
import Weight, { WeightSummary } from "./Weight";

/** Returns shelf's command at `path`, run `runs` times with `p95` bytes of output. */
function ran(path: string, runs: number, p95: number): Command {
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
  return { ...commandOf(shelf, path), cost: { figures, findings: null } };
}

const tool: Tool = {
  ...shelf,
  commands: [ran("books list", 842, 5920), ran("books show", 3, 300)],
};

const page = (path: string, bytes: number) => ({
  path: path === "" ? [] : path.split(" "),
  bytes,
  failure: null,
  exit: null,
});

const measured: WeightReply = {
  pages: [page("", 1420), page("books", 960), page("books list", 1180), page("books show", 860)],
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

function renderWeight() {
  render(
    <QueryClientProvider client={makeQueryClient()}>
      <span data-testid="summary">
        <WeightSummary tool={tool} />
      </span>
      <Weight tool={tool} />
    </QueryClientProvider>,
  );
}

/** Returns the table's top-level rows as their first cell's text and bytes. */
function rows(): string[][] {
  const table = screen.getByRole("table", { name: "Weight of each command's path" });
  return within(table)
    .getAllByRole("row")
    .slice(1)
    .map((row) =>
      [...row.querySelectorAll("td")].slice(0, 3).map((cell) => cell.textContent ?? ""),
    );
}

describe("Weight", () => {
  it("measures once, sorts paths heaviest first, and calls a path over budget in words", async () => {
    const count = stubWeight(measured);

    renderWeight();

    const table = await screen.findByRole("table", { name: "Weight of each command's path" });
    expect(rows()).toEqual([
      ["books list", "", "9,480"],
      ["books show", "", "3,540"],
    ]);
    expect(within(table).getByText("over by 1,480 B")).toHaveClass("text-destructive");
    expect(within(table).getByText("4,460 B under")).not.toHaveClass("text-destructive");
    expect(
      within(table).getByRole("img", { name: "9,480 B against a 8,000 B default budget" }),
    ).toBeInTheDocument();
    expect(screen.getByTestId("summary")).toHaveTextContent(
      "2 commands from 4 help pages measured when this view opened · 1 path over budget",
    );
    expect(count.asked).toBe(1);
  });

  it("opens a row to show each step, its budget and the heaviest step, ending with the path's sum", async () => {
    stubWeight(measured);
    renderWeight();
    const table = await screen.findByRole("table", { name: "Weight of each command's path" });

    await userEvent.click(within(table).getByRole("button", { name: "books list" }));

    expect(within(table).getByRole("button", { name: "books list" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    expect(
      rows()
        .slice(1, 6)
        .map((row) => row[0]),
    ).toEqual([
      "shelf -hroot page",
      "shelf books -hgroup page",
      "shelf books list -hcommand page",
      "outputp95 of real outputheaviest",
      "Path",
    ]);
    expect(within(table).getByText("over by 160 B")).toHaveClass("text-destructive");
    expect(within(table).getByText("counts toward the path only")).toBeInTheDocument();
  });

  it("marks a step that was not measured and calls its path partial, never over", async () => {
    stubWeight({
      ...measured,
      pages: measured.pages.map((each) =>
        each.path.join(" ") === "books"
          ? { ...each, bytes: null, failure: "timed_out" as const }
          : each,
      ),
    });
    renderWeight();
    const table = await screen.findByRole("table", { name: "Weight of each command's path" });

    await userEvent.click(within(table).getByRole("button", { name: "books list" }));

    expect(within(table).getByText("not measured: timed out")).toBeInTheDocument();
    expect(within(table).getAllByText("partial").length).toBeGreaterThan(0);
    expect(within(table).queryByText(/^over by 520 B/)).toBeNull();
  });

  it("says why when flightdeck could not measure the help pages, and offers to measure again", async () => {
    stubWeight({ pages: [], left_out: 0, problem: "cannot make a temporary state directory" });

    renderWeight();

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "flightdeck could not measure help pages: cannot make a temporary state directory",
    );
    expect(screen.getByRole("button", { name: "Measure again" })).toBeInTheDocument();
  });

  it("measures again when the person presses Measure again after a failed request", async () => {
    let answer: (reply: Response) => void = () => {};
    const replies: (() => Promise<Response>)[] = [
      () => Promise.reject(new TypeError("timed out")),
      () => new Promise<Response>((resolve) => (answer = resolve)),
    ];
    vi.stubGlobal("fetch", () => (replies.shift() ?? (() => Promise.reject()))());
    renderWeight();
    await screen.findByRole("alert");

    await userEvent.click(screen.getByRole("button", { name: "Measure again" }));

    expect(await screen.findByRole("status")).toHaveTextContent("Measuring shelf's help pages…");
    expect(screen.queryByRole("button", { name: "Measure again" })).toBeNull();
    answer(Response.json(measured));
    await screen.findByRole("table", { name: "Weight of each command's path" });
    expect(replies).toHaveLength(0);
  });

  it("shows the paths of a partial measurement under a line that says how many pages it missed", async () => {
    stubWeight({
      ...measured,
      pages: measured.pages.map((each) =>
        each.path.length === 2 ? { ...each, bytes: null, failure: "not_run" as const } : each,
      ),
    });

    renderWeight();

    await screen.findByRole("table", { name: "Weight of each command's path" });
    expect(screen.getByRole("status")).toHaveTextContent(
      "flightdeck stopped measuring before 2 help pages, at its 30 s limit or its own stop.",
    );
    expect(screen.getByRole("button", { name: "Measure again" })).toBeInTheDocument();
  });

  it("keeps the measurement going when the tab closes before the reply", async () => {
    const signals: (AbortSignal | null | undefined)[] = [];
    vi.stubGlobal("fetch", (_path: string, init: RequestInit) => {
      signals.push(init.signal);
      return new Promise<Response>(() => {});
    });
    const client = makeQueryClient();
    const { unmount } = render(
      <QueryClientProvider client={client}>
        <Weight tool={tool} />
      </QueryClientProvider>,
    );
    await screen.findByRole("status");

    unmount();

    expect(signals).toHaveLength(1);
    expect(signals[0]?.aborted).toBe(false);
  });

  it("asks once when the measurement fails, so a timeout does not start another", async () => {
    const count = { asked: 0 };
    vi.stubGlobal("fetch", async () => {
      count.asked += 1;
      throw new TypeError("timed out");
    });

    renderWeight();

    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(count.asked).toBe(1);
  });
});

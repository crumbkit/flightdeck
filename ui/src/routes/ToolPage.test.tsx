import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Command, ModelReply, Run } from "@/api/model";
import { makeQueryClient } from "@/queryClient";
import { shelfReply } from "../../tests/fixtures/shelf";
import { ToolPage } from "./ToolPage";

/** The stub server's answer to one route: the lines of a stream, or a failure. */
type Answer = { lines: object[] } | { status: number; problem: object };

/**
 * Stubs fetch with an answer for each route the test names, and returns
 * the path of every request the page sent.
 */
function stubServer(answers: Record<string, Answer>): string[] {
  const sent: string[] = [];
  vi.stubGlobal("fetch", async (path: string) => {
    sent.push(path);
    const answer = answers[path];
    if (answer === undefined) {
      return Response.json({ code: "test::unknown", title: path }, { status: 404 });
    }
    if ("status" in answer) {
      return Response.json(answer.problem, { status: answer.status });
    }
    return new Response(answer.lines.map((line) => `${JSON.stringify(line)}\n`).join(""), {
      headers: { "Content-Type": "application/x-ndjson" },
    });
  });
  return sent;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

/** Renders the page of `reply` as the app does, inside a query client. */
function renderPage(reply: ModelReply) {
  render(
    <QueryClientProvider client={makeQueryClient()}>
      <ToolPage reply={reply} onSignedOut={() => undefined} />
    </QueryClientProvider>,
  );
}

/** shelf's model with `books remove` changed by `change`. */
function withRemove(change: (command: Command) => Command): ModelReply {
  const commands = shelfReply.tool.commands.map((each) =>
    each.path.join(" ") === "books remove" ? change(each) : each,
  );
  return { ...shelfReply, tool: { ...shelfReply.tool, commands } };
}

const started = {
  event: "started",
  run: "r1",
  line: "shelf books remove 2",
  started_at: "2026-11-02T09:00:00Z",
};
const removed = JSON.stringify({
  schema: "shelf/books-remove/v1",
  data: [{ id: 2, title: "Frankenstein" }],
});
const confirmedRun = {
  lines: [
    started,
    { event: "line", text: removed },
    { event: "ended", outcome: { kind: "result" }, exit: 0, duration_ms: 3 },
  ],
};

/** Chooses books remove in the rail, enters 2 and presses Preview. */
async function previewRemove2() {
  const user = userEvent.setup();
  const rail = screen.getByRole("navigation", { name: "Commands" });
  await user.click(within(rail).getByRole("button", { name: /books remove/ }));
  await user.type(screen.getByLabelText(/The ids of the books to remove/), "2 ");
  await user.click(screen.getByRole("button", { name: /Preview/ }));
  return user;
}

describe("ToolPage", () => {
  it("says a command without a dry run has no preview, and confirms its request once", async () => {
    const sent = stubServer({
      "/api/previews": { lines: [{ ...started, run: null, preview: false, confirmation: "c1" }] },
      "/api/confirmations/c1": confirmedRun,
    });
    const reply = withRemove((command) => ({
      ...command,
      arguments: command.arguments.filter((each) => each.crumbkit_flag !== "--dry-run"),
    }));
    renderPage(reply);

    const user = await previewRemove2();
    const card = await screen.findByRole("region", { name: /^Run 1: .*awaiting confirmation/ });
    expect(within(card).getByText("This command has no preview.")).toBeInTheDocument();
    expect(
      within(card).getByRole("group", { name: "Run shelf books remove 2 --yes?" }),
    ).toBeVisible();
    await user.click(within(card).getByRole("button", { name: "Remove" }));

    expect(
      await screen.findByRole("region", { name: /^Run 1: shelf books remove 2, exit 0/ }),
    ).toBeInTheDocument();
    expect(sent.filter((path) => path === "/api/confirmations/c1")).toHaveLength(1);
  });

  it("shows the failure of a confirmation the server refuses and never sends it again", async () => {
    const sent = stubServer({
      "/api/previews": {
        lines: [
          { ...started, preview: true, confirmation: null },
          { event: "line", text: removed },
          {
            event: "ended",
            outcome: { kind: "result" },
            exit: 0,
            duration_ms: 3,
            confirmation: "c1",
          },
        ],
      },
      "/api/confirmations/c1": {
        status: 422,
        problem: {
          code: "flightdeck::run::confirmation",
          title: "The confirmation was already used.",
          fields: { reason: "used" },
        },
      },
    });
    renderPage(shelfReply);

    const user = await previewRemove2();
    const card = await screen.findByRole("region", { name: /^Run 1: .*awaiting confirmation/ });
    await user.click(within(card).getByRole("button", { name: "Remove 1 book" }));

    const refused = await screen.findByRole("region", { name: /^Run 1: .*refused/ });
    expect(within(refused).getByRole("alert")).toHaveTextContent("flightdeck::run::confirmation");
    expect(within(refused).queryByRole("button", { name: /Remove/ })).toBeNull();
    expect(sent.filter((path) => path.startsWith("/api/confirmations/"))).toHaveLength(1);
  });

  it("opens the preview card for a run that fails because it needs --yes", async () => {
    stubServer({
      "/api/runs": {
        lines: [
          started,
          {
            event: "ended",
            outcome: {
              kind: "tool_failure",
              failure: { code: "shelf::confirmation::needs_yes", message: "asks first" },
            },
            exit: 1,
            duration_ms: 3,
          },
        ],
      },
      "/api/previews": {
        lines: [
          { ...started, preview: true, confirmation: null },
          { event: "line", text: removed },
          {
            event: "ended",
            outcome: { kind: "result" },
            exit: 0,
            duration_ms: 3,
            confirmation: "c1",
          },
        ],
      },
    });
    const reply = withRemove((command) => ({
      ...command,
      class: "additive",
      effects: { read_only: false, destructive: false, idempotent: true },
    }));
    const user = userEvent.setup();
    renderPage(reply);

    const rail = screen.getByRole("navigation", { name: "Commands" });
    await user.click(within(rail).getByRole("button", { name: /books remove/ }));
    await user.type(screen.getByLabelText(/The ids of the books to remove/), "2 ");
    await user.click(screen.getByRole("button", { name: /Run/ }));

    const card = await screen.findByRole("region", { name: /^Run 2: .*awaiting confirmation/ });
    expect(within(card).getByRole("group", { name: "Remove 1 book?" })).toBeVisible();
    expect(within(card).getByRole("button", { name: "Cancel" })).toHaveFocus();
  });
  it("sends nothing when Enter submits a command that --read-only locks", async () => {
    const sent = stubServer({});
    // A single-value text field, where Enter submits the form.
    const reply = withRemove((command) => ({
      ...command,
      arguments: command.arguments.map((each) =>
        each.name === "<ID>"
          ? { ...each, arity: { kind: "one" as const, value_names: ["ID"] } }
          : each,
      ),
    }));
    const user = userEvent.setup();
    renderPage({ ...reply, read_only: true });

    const rail = screen.getByRole("navigation", { name: "Commands" });
    await user.click(within(rail).getByRole("button", { name: /books remove/ }));
    await user.type(screen.getByLabelText(/The ids of the books to remove/), "2{Enter}");

    expect(screen.queryByRole("region", { name: /^Run 1/ })).toBeNull();
    expect(sent.filter((path) => path === "/api/runs" || path === "/api/previews")).toEqual([]);
  });

  it("opens the usage view from the top bar on Figures, and Steps on the command that ran most", async () => {
    const run = (step: number, command: string): Run => ({
      session: 1,
      step,
      at: 1_794_000_000 + step,
      caller: "claude-code",
      command,
      flags: [],
      help: false,
      exit: 0,
      code: null,
      ms: 4,
      out_bytes: 12,
    });
    const history = [run(1, "books list"), run(2, "books show"), run(3, "books show")];
    renderPage({ ...shelfReply, tool: { ...shelfReply.tool, history } });
    const rail = screen.getByRole("navigation", { name: "Commands" });
    await userEvent.click(within(rail).getByRole("button", { name: /^books summarize/ }));
    const views = screen.getByRole("navigation", { name: "Views" });

    await userEvent.click(within(views).getByRole("button", { name: "Usage" }));

    const tabs = await screen.findByRole("navigation", { name: "Usage views" });
    expect(
      within(tabs)
        .getAllByRole("button")
        .map((each) => each.textContent),
    ).toEqual(["Figures", "Weight", "Steps", "Sessions", "Flags"]);
    expect(within(tabs).getByRole("button", { name: "Figures" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    expect(screen.getByText("shelf keeps no usage figures.")).toBeVisible();
    expect(screen.getByRole("form", { name: "Composer", hidden: true })).not.toBeVisible();

    await userEvent.click(within(tabs).getByRole("button", { name: "Steps" }));

    expect(
      screen.getByRole("heading", { name: "After each command, what the agent ran next" }),
    ).toBeVisible();
    expect(screen.getByRole("combobox", { name: "Command" })).toHaveValue("books show");

    await userEvent.click(within(views).getByRole("button", { name: "Commands" }));

    expect(screen.getByRole("form", { name: "Composer" })).toBeVisible();
    expect(screen.queryByRole("navigation", { name: "Usage views" })).toBeNull();
  });

  it("has a usage view of Figures and Weight alone for a tool with figures that lists no runs, and a command's name opens its form", async () => {
    const commands = shelfReply.tool.commands.map((each) =>
      each.path.join(" ") === "books show"
        ? {
            ...each,
            runs: 4,
            cost: {
              figures: {
                runs: 4,
                help_pages: 1,
                help_before: 1,
                refused: 0,
                failed: 2,
                p50_ms: 8,
                p95_ms: 30,
                p50_out_bytes: 400,
                p95_out_bytes: 900,
              },
              findings: null,
            },
          }
        : each,
    );
    renderPage({ ...shelfReply, tool: { ...shelfReply.tool, commands, history: null } });
    const views = screen.getByRole("navigation", { name: "Views" });

    await userEvent.click(within(views).getByRole("button", { name: "Usage" }));

    const tabs = await screen.findByRole("navigation", { name: "Usage views" });
    expect(
      within(tabs)
        .getAllByRole("button")
        .map((each) => each.textContent),
    ).toEqual(["Figures", "Weight"]);
    const table = screen.getByRole("table", { name: "Figures of each command" });

    await userEvent.click(within(table).getByRole("button", { name: "books show" }));

    expect(screen.getByRole("form", { name: "Composer" })).toBeVisible();
    expect(screen.getByRole("heading", { level: 2, name: "books show" })).toBeVisible();
  });

  it("has no usage view for a tool that reports no figures and lists no runs", () => {
    renderPage({ ...shelfReply, tool: { ...shelfReply.tool, history: null } });

    expect(screen.queryByRole("navigation", { name: "Views" })).toBeNull();
  });
});

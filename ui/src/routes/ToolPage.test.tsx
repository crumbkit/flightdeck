import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Command, ModelReply } from "@/api/model";
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
    render(<ToolPage reply={reply} onSignedOut={() => undefined} />);

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
    render(<ToolPage reply={shelfReply} onSignedOut={() => undefined} />);

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
      effects: { read_only: false, destructive: false, idempotent: true },
    }));
    const user = userEvent.setup();
    render(<ToolPage reply={reply} onSignedOut={() => undefined} />);

    const rail = screen.getByRole("navigation", { name: "Commands" });
    await user.click(within(rail).getByRole("button", { name: /books remove/ }));
    await user.type(screen.getByLabelText(/The ids of the books to remove/), "2 ");
    await user.click(screen.getByRole("button", { name: /Run/ }));

    const card = await screen.findByRole("region", { name: /^Run 2: .*awaiting confirmation/ });
    expect(within(card).getByRole("group", { name: "Remove 1 book?" })).toBeVisible();
    expect(within(card).getByRole("button", { name: "Cancel" })).toHaveFocus();
  });
});

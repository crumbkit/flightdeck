import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "@/App";
import { dropToken } from "@/api/client";
import { makeQueryClient } from "@/queryClient";
import { readToken } from "@/storage";
import { shelfReply } from "../tests/fixtures/shelf";

/** A request the page sent, as the stub of fetch saw it. */
type Sent = { path: string; method: string; headers: Headers; body: string | null };

/**
 * Stubs fetch with the server's routes: the session exchange, which accepts
 * only `code`, the model and a run of books show, with a keepalive line.
 * With `cut`, the run's stream closes before its ended line.
 */
function stubServer(code: string, cut = false): Sent[] {
  const sent: Sent[] = [];
  vi.stubGlobal("fetch", async (path: string, init: RequestInit) => {
    const body = typeof init.body === "string" ? init.body : null;
    sent.push({ path, method: init.method ?? "GET", headers: new Headers(init.headers), body });
    if (path === "/api/session") {
      return body === JSON.stringify({ code })
        ? Response.json({ token: "token-for-test" })
        : Response.json({ title: "refused" }, { status: 401 });
    }
    if (path === "/api/model") {
      return Response.json(shelfReply);
    }
    const lines = [
      {
        event: "started",
        run: "r1",
        line: "shelf books show 2",
        started_at: "2026-11-02T09:00:00Z",
      },
      { event: "alive" },
      {
        event: "line",
        text: JSON.stringify({
          schema: "shelf/books-show/v1",
          data: [{ id: 2, title: "Frankenstein" }],
          failures: [],
        }),
      },
      ...(cut ? [] : [{ event: "ended", outcome: { kind: "result" }, exit: 0, duration_ms: 12 }]),
    ];
    return new Response(lines.map((line) => `${JSON.stringify(line)}\n`).join(""), {
      headers: { "Content-Type": "application/x-ndjson" },
    });
  });
  return sent;
}

function renderApp(code: string | null) {
  return render(
    <QueryClientProvider client={makeQueryClient()}>
      <App code={code} />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  dropToken();
});

describe("App", () => {
  it("exchanges the launch code once, keeps the token for the tab and sends it on each request", async () => {
    const sent = stubServer("launch-code-for-test");

    renderApp("launch-code-for-test");

    expect(await screen.findByRole("region", { name: "Welcome to shelf" })).toBeInTheDocument();
    expect(readToken()).toBe("token-for-test");
    expect(sent.filter((each) => each.path === "/api/session")).toHaveLength(1);
    const model = sent.find((each) => each.path === "/api/model");
    expect(model?.headers.get("Authorization")).toBe("Bearer token-for-test");
  });

  it("draws the restart card with the command in a copy block for a refused code", async () => {
    stubServer("the-right-code");

    renderApp("an-expired-code");

    const card = await screen.findByRole("region", { name: "This page is not signed in" });
    expect(within(card).getByText("flightdeck open <TOOL>")).toBeInTheDocument();
    expect(within(card).getByRole("button", { name: "Copy" })).toBeInTheDocument();
  });

  it("runs the composer's command with structured values, never a command line", async () => {
    const sent = stubServer("launch-code-for-test");
    const user = userEvent.setup();
    renderApp("launch-code-for-test");

    await screen.findByRole("region", { name: "Welcome to shelf" });
    const rail = screen.getByRole("navigation", { name: "Commands" });
    await user.click(within(rail).getByRole("button", { name: /books show/ }));
    await user.type(screen.getByLabelText(/The ids of the books to show/), "2 ");
    await user.click(screen.getByRole("button", { name: /Run/ }));

    expect(
      await screen.findByRole("region", { name: /Run 1: shelf books show 2, exit 0/ }),
    ).toBeInTheDocument();
    const run = sent.find((each) => each.path === "/api/runs");
    expect(run?.method).toBe("POST");
    expect(run?.headers.get("Content-Type")).toBe("application/json");
    expect(JSON.parse(run?.body ?? "null")).toEqual({
      path: ["books", "show"],
      values: { "<ID>": ["2"] },
    });
  });

  it("shows a run whose stream closes before its ended line as cut off", async () => {
    stubServer("launch-code-for-test", true);
    const user = userEvent.setup();
    renderApp("launch-code-for-test");

    await screen.findByRole("region", { name: "Welcome to shelf" });
    const rail = screen.getByRole("navigation", { name: "Commands" });
    await user.click(within(rail).getByRole("button", { name: /books show/ }));
    await user.type(screen.getByLabelText(/The ids of the books to show/), "2 ");
    await user.click(screen.getByRole("button", { name: /Run/ }));

    const card = await screen.findByRole("region", { name: /Run 1: shelf books show 2, cut off/ });
    expect(within(card).getByRole("alert")).toHaveTextContent(/stopped before the run ended/);
  });
});

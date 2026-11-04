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
 * only `code`, the model, a cancel, and a run of books show, with a
 * keepalive line. With `cut`, the run's stream closes before its ended
 * line. With `runBody`, the run answers with that stream instead.
 */
function stubServer(
  code: string,
  cut = false,
  runStatus = 200,
  runBody: ReadableStream<Uint8Array> | null = null,
): Sent[] {
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
    if (path.endsWith("/cancel")) {
      return new Response(null, { status: 204 });
    }
    // The IDs chooser asks for its choices, which these tests leave unlisted.
    if (path === "/api/choices") {
      return Response.json({ code: "test::no_choices", title: path }, { status: 404 });
    }
    if (runBody !== null) {
      return new Response(runBody, { headers: { "Content-Type": "application/x-ndjson" } });
    }
    if (runStatus !== 200) {
      return Response.json(
        { code: "flightdeck::session::unauthorized", title: "The request needs the page's token." },
        { status: runStatus },
      );
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

/**
 * Returns a run's stream that sends `started` with the id r1 and the line
 * of books show 2 with its result, and then breaks, or, with `open`, stays
 * open until the test ends.
 */
function streamThen(end: "break" | "open"): ReadableStream<Uint8Array> {
  const lines = [
    { event: "started", run: "r1", line: "shelf books show 2", started_at: "2026-11-02T09:00:00Z" },
    {
      event: "line",
      text: JSON.stringify({
        schema: "shelf/books-show/v1",
        data: [{ id: 2, title: "Frankenstein" }],
      }),
    },
  ];
  const bytes = new TextEncoder().encode(lines.map((line) => `${JSON.stringify(line)}\n`).join(""));
  let pulls = 0;
  return new ReadableStream({
    pull(controller) {
      pulls += 1;
      if (pulls === 1) {
        controller.enqueue(bytes);
      } else if (end === "break") {
        controller.error(new TypeError("network error"));
      }
    },
  });
}

/** Opens books show in the composer, enters 2 and presses Run. */
async function runBooksShow(user: ReturnType<typeof userEvent.setup>) {
  await screen.findByRole("region", { name: "Welcome to shelf" });
  const rail = screen.getByRole("navigation", { name: "Commands" });
  await user.click(within(rail).getByRole("button", { name: /books show/ }));
  await user.type(screen.getByLabelText(/The ids of the books to show/), "2 ");
  await user.click(screen.getByRole("button", { name: /Run/ }));
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
    const kept = Object.keys(window.localStorage).map((key) => window.localStorage.getItem(key));
    expect(kept.join(" ")).not.toContain("token-for-test");
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

  it("signs the page out when the server refuses the token during a run", async () => {
    stubServer("launch-code-for-test", false, 401);
    const user = userEvent.setup();
    renderApp("launch-code-for-test");

    await screen.findByRole("region", { name: "Welcome to shelf" });
    const rail = screen.getByRole("navigation", { name: "Commands" });
    await user.click(within(rail).getByRole("button", { name: /books show/ }));
    await user.type(screen.getByLabelText(/The ids of the books to show/), "2 ");
    await user.click(screen.getByRole("button", { name: /Run/ }));

    expect(
      await screen.findByRole("region", { name: "This page is not signed in" }),
    ).toBeInTheDocument();
    expect(readToken()).toBeNull();
  });

  it("keeps the rows drawn when the connection breaks mid-run, and says that it broke", async () => {
    stubServer("launch-code-for-test", false, 200, streamThen("break"));
    const user = userEvent.setup();
    renderApp("launch-code-for-test");

    await runBooksShow(user);

    const card = await screen.findByRole("region", { name: /Run 1: shelf books show 2, cut off/ });
    expect(within(card).getByText("Frankenstein")).toBeInTheDocument();
    expect(within(card).getByRole("alert")).toHaveTextContent(
      /connection to flightdeck broke before the run ended/,
    );
    expect(within(card).getByRole("alert")).not.toHaveTextContent(/[Rr]eload/);
  });

  it("never cancels a run on Esc, and cancels the newest running run on ⌘.", async () => {
    const sent = stubServer("launch-code-for-test", false, 200, streamThen("open"));
    const user = userEvent.setup();
    renderApp("launch-code-for-test");
    await runBooksShow(user);
    await screen.findByRole("button", { name: "Cancel" });

    await user.keyboard("{Escape}");
    expect(sent.filter((each) => each.path.endsWith("/cancel"))).toHaveLength(0);
    await user.keyboard("{Meta>}.{/Meta}");

    const cancels = sent.filter((each) => each.path.endsWith("/cancel"));
    expect(cancels.map((each) => [each.method, each.path])).toEqual([
      ["POST", "/api/runs/r1/cancel"],
    ]);
  });
});

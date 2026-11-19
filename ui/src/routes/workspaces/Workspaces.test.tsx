import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { makeQueryClient } from "@/queryClient";
import { workspacesList } from "../../../tests/fixtures/workspaces";
import { Workspaces } from "./Workspaces";

/** Answers `GET /api/workspaces` with each of `replies` in turn, the last one again after. */
function stubWorkspaces(...replies: Response[]) {
  const asked = vi.fn();
  vi.stubGlobal("fetch", async (path: string) => {
    asked(path);
    return (replies.length > 1 ? replies.shift() : replies[0])?.clone();
  });
  return asked;
}

function renderList() {
  return render(
    <QueryClientProvider client={makeQueryClient()}>
      <Workspaces onSignedOut={() => {}} />
    </QueryClientProvider>,
  );
}

/** Returns the card of the workspace `name`. */
async function card(name: string): Promise<HTMLElement> {
  return screen.findByRole("link", { name: new RegExp(`^${name}`) });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("Workspaces", () => {
  it("heads the page with the target and the running and stopped counts", async () => {
    stubWorkspaces(Response.json(workspacesList));
    renderList();

    expect(await screen.findByText("1 running")).toBeVisible();
    expect(screen.getByRole("heading", { level: 1, name: "Workspaces on local" })).toBeVisible();
    expect(screen.getByText("Each card is one mxv workspace.")).toBeVisible();
    expect(screen.getByText("1 stopped")).toBeVisible();
  });

  it("draws one card per workspace that links to its page", async () => {
    stubWorkspaces(Response.json(workspacesList));
    renderList();

    const reading = await card("reading-list");
    expect(reading).toHaveAttribute("href", "#/workspaces/reading-list");
    expect(screen.getAllByRole("link", { name: /-/ })).toHaveLength(4);
  });

  it("gives a card its branch, repos, units, routes and whether it is pinned", async () => {
    stubWorkspaces(Response.json(workspacesList));
    renderList();

    const reading = within(await card("reading-list"));
    expect(reading.getByText("reading, shelf")).toBeVisible();
    expect(reading.getByText("2 of 3 running")).toBeVisible();
    expect(reading.getByText("web.reading-list.mxv.localhost")).toBeVisible();
    expect(reading.getByText("Pinned: no")).toBeVisible();
    const notes = within(await card("notes-scratch"));
    expect(notes.getByText("none declared")).toBeVisible();
    expect(notes.getByText("none")).toBeVisible();
    expect(notes.getByText("Pinned: yes")).toBeVisible();
  });

  it("falls back to the host ports when a workspace has no route", async () => {
    stubWorkspaces(Response.json(workspacesList));
    renderList();

    const shelf = within(await card("shelf-import"));
    expect(shelf.getByText("127.0.0.1:41010")).toBeVisible();
    expect(shelf.getByText("stopped")).toBeVisible();
  });

  it("says units unknown with mxv's code on the card whose status failed", async () => {
    stubWorkspaces(Response.json(workspacesList));
    renderList();

    const broken = within(await card("broken-status"));
    expect(broken.getByText("units unknown · mxv::manifest::invalid")).toBeVisible();
    expect(broken.getByText("Pinned: unknown")).toBeVisible();
  });

  it("says why no card shows a route when this mxv has no proxy status", async () => {
    const withoutProxy = {
      ...workspacesList,
      summary: {
        ...workspacesList.summary,
        routes: { state: "unavailable", reason: "this mxv has no proxy status command" },
      },
    };
    stubWorkspaces(Response.json(withoutProxy));
    renderList();

    expect(
      await screen.findByText(
        "Routes are unavailable: this mxv has no proxy status command. Cards show host ports.",
      ),
    ).toBeVisible();
  });

  it("draws an empty state when mxv lists no workspace", async () => {
    const empty = {
      ...workspacesList,
      data: [],
      summary: { ...workspacesList.summary, workspaces: 0, running: 0, stopped: 0 },
    };
    stubWorkspaces(Response.json(empty));
    renderList();

    expect(await screen.findByText("mxv lists no workspace.")).toBeVisible();
    expect(screen.getByText("mxv new <name> --repo <path>")).toBeVisible();
  });

  it("draws a failure card with Try again when mxv list fails, and asks again on Try again", async () => {
    const failed = Response.json(
      {
        schema: "flightdeck/failure/v1",
        code: "flightdeck::workspaces::mxv_failed",
        message: "mxv list failed: cannot read the workspace records",
        recovery: { kind: "retry" },
      },
      { status: 502 },
    );
    const asked = stubWorkspaces(failed, Response.json(workspacesList));
    renderList();

    expect(
      await screen.findByText("mxv list failed: cannot read the workspace records"),
    ).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));

    expect(await card("reading-list")).toBeVisible();
    expect(asked).toHaveBeenCalledTimes(2);
  });
});

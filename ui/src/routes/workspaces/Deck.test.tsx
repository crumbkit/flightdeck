import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { makeQueryClient } from "@/queryClient";
import { readingList, workspacesList } from "../../../tests/fixtures/workspaces";
import Deck from "./Deck";

/** Answers the workspaces routes with the fake mxv's documents, and records each path. */
function stubServer() {
  const asked = vi.fn();
  vi.stubGlobal("fetch", async (path: string) => {
    asked(path);
    return path === "/api/workspaces" ? Response.json(workspacesList) : Response.json(readingList);
  });
  return asked;
}

function renderDeck(props: Parameters<typeof Deck>[0]) {
  return render(
    <QueryClientProvider client={makeQueryClient()}>
      <Deck {...props} />
    </QueryClientProvider>,
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("Deck", () => {
  it("shows the workspaces list at the start when flightdeck holds no tool", async () => {
    stubServer();
    renderDeck({ place: { view: "start" }, tool: false, onSignedOut: () => {} });

    expect(await screen.findByText("1 running")).toBeVisible();
    const views = screen.getByRole("navigation", { name: "Views" });
    const workspaces = within(views).getByRole("link", { name: /^Workspaces/ });
    expect(workspaces).toHaveAttribute("aria-current", "page");
    expect(workspaces).toHaveTextContent("Workspaces4");
    expect(within(views).queryByRole("link", { name: "Commands" })).toBeNull();
  });

  it("names the local target with its count of workspaces", async () => {
    stubServer();
    renderDeck({ place: { view: "workspaces" }, tool: false, onSignedOut: () => {} });

    const targets = screen.getByRole("navigation", { name: "Target" });
    expect(await within(targets).findByText("4")).toBeVisible();
    expect(within(targets).getByRole("link", { name: /local · this Mac/ })).toHaveAttribute(
      "href",
      "#/workspaces",
    );
  });

  it("links back to the tool's commands when flightdeck holds a tool", async () => {
    stubServer();
    renderDeck({ place: { view: "workspaces" }, tool: true, onSignedOut: () => {} });

    const views = screen.getByRole("navigation", { name: "Views" });
    expect(within(views).getByRole("link", { name: "Commands" })).toHaveAttribute("href", "#/");
    await screen.findByText("1 running");
  });

  it("shows one workspace without asking for the whole list", async () => {
    const asked = stubServer();
    renderDeck({
      place: { view: "workspace", name: "reading-list" },
      tool: false,
      onSignedOut: () => {},
    });

    expect(await screen.findByRole("table", { name: "Units" })).toBeVisible();
    expect(asked).toHaveBeenCalledTimes(1);
    expect(asked).toHaveBeenCalledWith("/api/workspaces/reading-list");
  });
});

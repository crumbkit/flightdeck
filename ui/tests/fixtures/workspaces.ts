import list from "./workspaces-list.json" with { type: "json" };
import show from "./workspaces-show.json" with { type: "json" };

/**
 * The fake mxv's documents as `GET /api/workspaces` and
 * `GET /api/workspaces/reading-list` send them. They are copies of
 * tests/golden/workspaces/list.json and show.json, which the server's
 * tests compare with what it sends, and the acceptance journey compares
 * them with the running server's replies.
 */
export const workspacesList = list;
export const readingList = show;

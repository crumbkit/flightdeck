import list from "./workflows-list.json" with { type: "json" };
import show from "./workflows-show.json" with { type: "json" };
import tangled from "./workflows-show-tangled.json" with { type: "json" };

/**
 * The fake board's documents as `GET /api/workflows` and
 * `GET /api/workflows/development-flow` and `GET /api/workflows/tangled`
 * send them. They are copies of tests/golden/workflows/list.json,
 * show-development-flow.json and show-tangled.json, which
 * the server's tests compare with what it sends, and the acceptance
 * journey compares them with the running server's replies.
 */
export const workflowsList = list;
export const developmentFlow = show;
export const tangledFlow = tangled;

/**
 * The development flow as the page would get it once Board names the
 * running version of a flow: a newer version than the one the graph reads.
 * flightdeck sends `running` as null until then, so only the unit tests
 * use this document.
 */
export const developmentFlowOnOlderFlow = {
  ...show,
  data: { ...show.data, running: "/flows/development/7c1e04b29d3a5f60" },
};

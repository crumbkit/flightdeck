import { useSyncExternalStore } from "react";

/**
 * Where the page is: the start, which is the tool's page when flightdeck
 * holds a tool and the workspaces list otherwise, the workspaces list, one
 * workspace, or the Workflows page. The Workflows page names a graph by
 * its slug, or the first graph when it names none, and may name what is
 * selected: a node's id, a group's id or a change's path, the last for a
 * change page's link. The address's fragment holds it, so the browser's
 * back button and a link work.
 */
export type Place =
  | { view: "start" }
  | { view: "workspaces" }
  | { view: "workspace"; name: string }
  | { view: "workflows"; slug: string | null; selected: string | null };

const prefix = "#/workspaces";

const workflowsPrefix = "#/workflows";

/** Returns the Workflows place of the fragment's rest after `#/workflows`. */
function workflowsOf(rest: string): Place {
  const [path = "", query = ""] = rest.split("?", 2);
  const slug = path.replace(/^\//, "");
  const selected = new URLSearchParams(query).get("selected");
  return {
    view: "workflows",
    slug: /^[a-z0-9]+(-[a-z0-9]+)*$/.test(slug) ? slug : null,
    selected: selected === "" ? null : selected,
  };
}

/** Returns the place that the fragment `hash` names. */
export function placeOf(hash: string): Place {
  const workflows = /^#\/workflows([/?].*)?$/.exec(hash);
  if (workflows !== null) {
    return workflowsOf(workflows[1] ?? "");
  }
  const match = /^#\/workspaces(\/.*)?$/.exec(hash);
  if (match === null) {
    return { view: "start" };
  }
  try {
    const name = decodeURIComponent(match[1]?.slice(1) ?? "");
    return name === "" ? { view: "workspaces" } : { view: "workspace", name };
  } catch {
    return { view: "workspaces" };
  }
}

/** Returns the fragment of `place`, for a link's `href`. */
export function hrefOf(place: Place): string {
  if (place.view === "workflows") {
    const path = place.slug === null ? workflowsPrefix : `${workflowsPrefix}/${place.slug}`;
    return place.selected === null
      ? path
      : `${path}?${new URLSearchParams({ selected: place.selected }).toString()}`;
  }
  if (place.view === "workspace") {
    return `${prefix}/${encodeURIComponent(place.name)}`;
  }
  return place.view === "start" ? "#/" : prefix;
}

function subscribe(changed: () => void): () => void {
  window.addEventListener("hashchange", changed);
  return () => window.removeEventListener("hashchange", changed);
}

/** Returns the place that the address names now, and draws again when it changes. */
export function usePlace(): Place {
  const hash = useSyncExternalStore(subscribe, () => window.location.hash);
  return placeOf(hash);
}

/** Moves the page to `place`, as a link to it does. */
export function go(place: Place): void {
  window.location.hash = hrefOf(place);
}

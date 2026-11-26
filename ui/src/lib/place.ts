import { useSyncExternalStore } from "react";

/**
 * Where the page is: the start, which is the tool's page when flightdeck
 * holds a tool and the workspaces list otherwise, the workspaces list, or
 * one workspace. The address's fragment holds it, so the browser's back
 * button and a link work.
 */
export type Place =
  | { view: "start" }
  | { view: "workspaces" }
  | { view: "workspace"; name: string };

const prefix = "#/workspaces";

/** Returns the place that the fragment `hash` names. */
export function placeOf(hash: string): Place {
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

// The Compliance site's address, held in the page's hash so the one file
// needs no server: "#/" is the Compliance page, "#/rules" the Rules page and
// "#/repositories/<name>" one repository's page, each with its filters as a
// query, such as "#/rules?kind=standing". The browser stores nothing else.
import type { AddressSource } from "@crumbkit/ui/react";
import { useCallback, useSyncExternalStore } from "react";

/** One page of the site. */
export type Route =
  | { readonly page: "compliance" }
  | { readonly page: "rules" }
  | { readonly page: "repository"; readonly name: string };

/** Where the site is: its page and the query of that page's filters. */
export interface Place {
  readonly route: Route;
  /** The query, with its leading "?", or "" for none. */
  readonly search: string;
}

const REPOSITORY = "/repositories/";

/** Returns the place `hash` names, such as "#/rules?kind=standing"; any hash the site does not know is the Compliance page. */
export function parsePlace(hash: string): Place {
  const text = hash.startsWith("#") ? hash.slice(1) : hash;
  const at = text.indexOf("?");
  const path = at === -1 ? text : text.slice(0, at);
  const search = at === -1 ? "" : text.slice(at);
  if (path === "/rules") return { route: { page: "rules" }, search };
  if (path.startsWith(REPOSITORY) && path.length > REPOSITORY.length) {
    try {
      const name = decodeURIComponent(path.slice(REPOSITORY.length));
      return { route: { page: "repository", name }, search };
    } catch {
      return { route: { page: "compliance" }, search: "" };
    }
  }
  return { route: { page: "compliance" }, search: path === "/" || path === "" ? search : "" };
}

/** Returns the hash of `route` with `search`, such as "#/repositories/owner%2Fapp?rule=%2Frules%2Fa". */
export function placeHash(route: Route, search = ""): string {
  switch (route.page) {
    case "compliance":
      return `#/${search}`;
    case "rules":
      return `#/rules${search}`;
    case "repository":
      return `#${REPOSITORY}${encodeURIComponent(route.name)}${search}`;
  }
}

/** Joins queries that hold different keys, such as "?show=not_current" and "?group=Platform". */
export function joinSearch(...searches: readonly string[]): string {
  const parts = searches
    .map((search) => (search.startsWith("?") ? search.slice(1) : search))
    .filter((part) => part !== "");
  return parts.length === 0 ? "" : `?${parts.join("&")}`;
}

/**
 * Returns an address source for one set of filters on a page whose query
 * also holds `others`, the query of the page's other filters, so a change
 * to one set keeps the rest.
 */
export function sharedSource(
  search: string,
  others: string,
  navigate: (search: string) => void,
): AddressSource {
  return { search, navigate: (next) => navigate(joinSearch(next, others)) };
}

function subscribe(onChange: () => void): () => void {
  window.addEventListener("hashchange", onChange);
  return () => window.removeEventListener("hashchange", onChange);
}

/** Returns the site's place, which changes with the hash, and a function that moves to another query on the same page. */
export function usePlace(): { readonly place: Place; readonly navigate: (search: string) => void } {
  const hash = useSyncExternalStore(subscribe, () => window.location.hash);
  const place = parsePlace(hash);
  const route = place.route;
  const key = placeHash(route);
  const navigate = useCallback(
    (search: string) => {
      window.location.hash = `${key}${search}`;
    },
    [key],
  );
  return { place, navigate };
}

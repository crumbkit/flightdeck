/**
 * Reads the launch code from the page's address, `#code=<code>`, and
 * removes the fragment with `history.replaceState`, so the code leaves the
 * address bar, the history and any later copy of the address before the
 * first paint. Returns null when the address holds no code.
 */
export function takeLaunchCode(location: Location, history: History): string | null {
  const code = new URLSearchParams(location.hash.slice(1)).get("code");
  if (location.hash !== "") {
    history.replaceState(history.state, "", `${location.pathname}${location.search}`);
  }
  return code === null || code === "" ? null : code;
}

/**
 * Reads the launch code from the page's address, `#code=<code>`, and
 * removes that fragment with `history.replaceState`, so the code leaves the
 * address bar, the history and any later copy of the address before the
 * first paint. A fragment without a code, such as a place on the page,
 * stays, so a reload keeps the place. Returns null when the address holds
 * no code.
 */
export function takeLaunchCode(location: Location, history: History): string | null {
  const code = new URLSearchParams(location.hash.slice(1)).get("code");
  if (code !== null) {
    history.replaceState(history.state, "", `${location.pathname}${location.search}`);
  }
  return code === null || code === "" ? null : code;
}

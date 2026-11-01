import { z } from "@/api/zod";

// The storage owner (BASE-7). The page keeps two values in the browser: the
// session's token in sessionStorage, which the browser drops when the tab
// closes, and the theme choice in localStorage. Nothing else reads or
// writes storage. A browser that refuses storage, as some private windows
// do, gives no token and the system theme.

const tokenKey = "flightdeck.token";
const themeKey = "flightdeck.theme";

/** The person's choice of appearance. */
export const Theme = z.enum(["system", "light", "dark"]);
export type Theme = z.infer<typeof Theme>;

/** Returns the session's token, or null when the page holds none. */
export function readToken(): string | null {
  try {
    // biome-ignore lint/style/noRestrictedGlobals: this module owns storage (BASE-7)
    return sessionStorage.getItem(tokenKey);
  } catch {
    return null;
  }
}

/** Keeps the session's token for this tab. */
export function writeToken(token: string): void {
  try {
    // biome-ignore lint/style/noRestrictedGlobals: this module owns storage (BASE-7)
    sessionStorage.setItem(tokenKey, token);
  } catch {
    // A tab that refuses storage keeps the token only until it reloads.
  }
}

/** Forgets the session's token, as when the server refuses it. */
export function forgetToken(): void {
  try {
    // biome-ignore lint/style/noRestrictedGlobals: this module owns storage (BASE-7)
    sessionStorage.removeItem(tokenKey);
  } catch {
    // Nothing was stored.
  }
}

/** Returns the stored theme choice, or "system" when there is none. */
export function readTheme(): Theme {
  try {
    // biome-ignore lint/style/noRestrictedGlobals: this module owns storage (BASE-7)
    const parsed = Theme.safeParse(localStorage.getItem(themeKey));
    return parsed.success ? parsed.data : "system";
  } catch {
    return "system";
  }
}

/** Stores the theme choice. */
export function writeTheme(theme: Theme): void {
  try {
    // biome-ignore lint/style/noRestrictedGlobals: this module owns storage (BASE-7)
    localStorage.setItem(themeKey, theme);
  } catch {
    // The choice then lasts until the page reloads.
  }
}

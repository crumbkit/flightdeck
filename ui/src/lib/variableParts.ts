// The page's first load reads this module, for a failure card's links, so it
// stays apart from the environment view's own module, which loads later.

/** One part of a sentence: plain text, or the name of a declared variable. */
export type Part = { text: string; variable: boolean };

/**
 * Splits `text` into its parts, with each whole-word occurrence of one of
 * `names` as a part of its own. A longer name wins over one that starts it,
 * so SHELF_PORT_RANGE is never cut to SHELF_PORT.
 */
export function partsOf(text: string, names: string[]): Part[] {
  if (names.length === 0 || text === "") {
    return [{ text, variable: false }];
  }
  const escaped = [...names]
    .sort((a, b) => b.length - a.length)
    .map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"));
  const pattern = new RegExp(`(?<![A-Za-z0-9_])(${escaped.join("|")})(?![A-Za-z0-9_])`, "g");
  const parts: Part[] = [];
  let at = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > at) {
      parts.push({ text: text.slice(at, match.index), variable: false });
    }
    parts.push({ text: match[0], variable: true });
    at = match.index + match[0].length;
  }
  if (at < text.length) {
    parts.push({ text: text.slice(at), variable: false });
  }
  return parts;
}

/**
 * One word of a line: its value after quoting is removed, where it starts
 * and ends in the line, and whether any part of it was quoted.
 */
export type Token = { value: string; start: number; end: number; quoted: boolean };

/** The words of a line, and the place of a quote that is not closed, or null. */
export type Tokenized = { tokens: Token[]; open: number | null };

/** The characters that a backslash escapes inside double quotes. */
const escapedInDoubleQuotes = ['"', "\\", "$", "`"];

/**
 * Splits a line into words as a POSIX shell does before it expands
 * anything: spaces and tabs separate words, single quotes keep everything
 * as written, double quotes keep everything except a backslash before
 * `"`, `\`, `$` or a backtick, and a backslash outside quotes keeps the
 * next character.
 *
 * The line never runs through a shell, so `$HOME`, `~` and `*` stay as
 * written. A backslash at the end of the line, which a shell reads as a
 * continuation, stays a backslash, because the line has one line.
 */
export function tokenize(text: string): Tokenized {
  const tokens: Token[] = [];
  let open: number | null = null;
  let current: Token | null = null;
  let index = 0;
  while (index < text.length) {
    const character = text[index] ?? "";
    if (character === " " || character === "\t") {
      if (current !== null) {
        tokens.push({ ...current, end: index });
        current = null;
      }
      index += 1;
      continue;
    }
    current ??= { value: "", start: index, end: index, quoted: false };
    const token = current;
    if (character === "\\") {
      const next = text[index + 1];
      token.value += next ?? "\\";
      index += next === undefined ? 1 : 2;
    } else if (character === "'" || character === '"') {
      token.quoted = true;
      const closing = readQuoted(text, index, token);
      if (closing === null) {
        open = index;
        index = text.length;
      } else {
        index = closing + 1;
      }
    } else {
      token.value += character;
      index += 1;
    }
  }
  if (current !== null) {
    tokens.push({ ...current, end: text.length });
  }
  return { tokens, open };
}

/**
 * Adds the quoted part that starts at `start` to `token`, and returns the
 * place of its closing quote, or null when the line ends first.
 */
function readQuoted(text: string, start: number, token: Token): number | null {
  const quote = text[start];
  let index = start + 1;
  while (index < text.length) {
    const character = text[index] ?? "";
    if (character === quote) {
      return index;
    }
    const next = text[index + 1];
    if (
      quote === '"' &&
      character === "\\" &&
      next !== undefined &&
      escapedInDoubleQuotes.includes(next)
    ) {
      token.value += next;
      index += 2;
    } else {
      token.value += character;
      index += 1;
    }
  }
  return null;
}

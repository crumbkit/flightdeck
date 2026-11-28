/** The characters that a POSIX shell reads as part of a plain word. */
const PLAIN = /^[A-Za-z0-9_./:@%+=,-]+$/;

/** Returns whether `text` holds a C0 control character, DEL, or a C1 control character. */
function hasControl(text: string): boolean {
  return [...text].some((character) => {
    const code = character.codePointAt(0) ?? 0;
    return code < 0x20 || (code >= 0x7f && code <= 0x9f);
  });
}

/**
 * Returns `text` as one POSIX shell word, for sh, bash or zsh: as it is when every character is
 * plain, and otherwise in single quotes, with each single quote written as
 * `'\''`, so a pasted command passes it as one argument. fish reads `\\` and `\'`
 * inside single quotes as escapes, so there a path holding a backslash
 * pastes as a different path.
 */
export function shellWord(text: string): string {
  return PLAIN.test(text) ? text : `'${text.replaceAll("'", "'\\''")}'`;
}

/**
 * Returns the POSIX shell command that reads the board record at `path`,
 * or null when the path holds a C0, DEL or C1 control character. Such a
 * character, such as the escape that ends a terminal's bracketed paste,
 * could make a pasted command run text the person did not see, so the page
 * copies no command for it.
 */
export function boardGet(path: string): string | null {
  return hasControl(path) ? null : `board get ${shellWord(path)}`;
}

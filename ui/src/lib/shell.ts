/** The characters that a POSIX shell reads as part of a plain word. */
const PLAIN = /^[A-Za-z0-9_./:@%+=,-]+$/;

/**
 * Returns `text` as one POSIX shell word: as it is when every character is
 * plain, and otherwise in single quotes, with each single quote written as
 * `'\''`, so a pasted command passes it as one argument.
 */
export function shellWord(text: string): string {
  return PLAIN.test(text) ? text : `'${text.replaceAll("'", "'\\''")}'`;
}

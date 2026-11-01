// An ISO 8601 duration, such as PT1H40M: years, months, weeks and days,
// then T and hours, minutes and seconds. Only the seconds take a fraction.
const pattern =
  /^P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)W)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+(?:\.\d+)?)S)?)?$/;

const units = ["y", "mo", "w", "d", "h", "min", "s"] as const;

/**
 * Returns an ISO 8601 duration in words, such as "1 h 40 min" for
 * `PT1H40M`, or null when `text` is not a duration. Each unit keeps the
 * number the tool printed, so `PT90M` reads "90 min".
 */
export function durationWords(text: string): string | null {
  const match = pattern.exec(text);
  if (match === null || text.endsWith("T")) {
    return null;
  }
  const words = units.flatMap((unit, index) => {
    const amount = match[index + 1];
    return amount === undefined ? [] : [`${amount} ${unit}`];
  });
  return words.length === 0 ? null : words.join(" ");
}

/** Returns how long a run took in words, such as "412 ms" or "2.4 s". */
export function elapsedWords(milliseconds: number): string {
  return milliseconds < 1000
    ? `${Math.round(milliseconds)} ms`
    : `${(milliseconds / 1000).toFixed(1)} s`;
}

/**
 * Returns a count with its noun, singular for 1, such as "1 run" or
 * "1,384 runs". The plural is the noun with an s unless `plural` says
 * otherwise.
 */
export function counted(count: number, noun: string, plural = `${noun}s`): string {
  return `${count.toLocaleString("en")} ${count === 1 ? noun : plural}`;
}

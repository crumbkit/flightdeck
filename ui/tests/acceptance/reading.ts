import { execFile } from "node:child_process";
import { constants } from "node:fs";
import { chmod, type FileHandle, mkdir, open, writeFile } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";

/** Writes a reading list named `name` with `entries`, one on each line. */
export async function writeList(data: string, name: string, entries: string[]): Promise<void> {
  const dir = path.join(data, "reading", "lists");
  await mkdir(dir, { recursive: true });
  await writeFile(path.join(dir, `${name}.txt`), `${entries.join("\n")}\n`);
}

/** A reading whose `entries stream` prints a line only when the test releases it. */
export type SlowReading = {
  /** The program to open: a script that runs reading. */
  program: string;
  /** Lets the stream print its next line. */
  release: () => Promise<void>;
  /** Closes the gate, which ends a stream that still waits. */
  close: () => Promise<void>;
};

/** Quotes a path for the script. */
function quoted(text: string): string {
  return `'${text.replaceAll("'", "'\\''")}'`;
}

/**
 * Writes a script named reading under `home` that runs `real` and passes
 * every command through, except `entries stream`, whose lines it holds
 * until the test releases each one through a named pipe. reading prints a
 * short list at once, so the hold is what lets a test see rows arrive, and
 * cancel a run, while the run is still going.
 */
export async function slowReading(home: string, real: string): Promise<SlowReading> {
  const bin = path.join(home, "bin");
  await mkdir(bin, { recursive: true });
  const gate = path.join(home, "gate");
  await promisify(execFile)("mkfifo", [gate]);
  const program = path.join(bin, "reading");
  await writeFile(
    program,
    `#!/bin/sh
case "$*" in
  "entries stream "*) ;;
  *) exec ${quoted(real)} "$@" ;;
esac
exec 3<${quoted(gate)}
${quoted(real)} "$@" | while IFS= read -r line; do
  read -r _ <&3 || exit 1
  printf '%s\\n' "$line"
done
`,
  );
  await chmod(program, 0o755);
  let writer: Promise<FileHandle> | null = null;
  return {
    program,
    release: async () => {
      writer ??= open(gate, "w");
      await (await writer).write("\n");
    },
    close: async () => {
      // A reader that does not wait lets a writer that is still opening the
      // pipe finish, when no stream ever opened the gate.
      const reader = await open(gate, constants.O_RDONLY | constants.O_NONBLOCK);
      await (await writer)?.close();
      await reader.close();
    },
  };
}

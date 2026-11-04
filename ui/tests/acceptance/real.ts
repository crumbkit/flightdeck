import { type ChildProcess, spawn } from "node:child_process";
import { once } from "node:events";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createInterface } from "node:readline";
import { pathToFileURL } from "node:url";
import { test as checked } from "./checks";
import { slowReading } from "./reading";

/** The repository's root, which holds the built flightdeck and shelf. */
const root = path.resolve(import.meta.dirname, "../../..");

/** How long flightdeck may take to print its two lines. */
const startLimitMs = 10_000;

/** A running `flightdeck open <tool> --no-browser`. */
export type Served = {
  /** The opener page, as a file URL. Opening it signs the page in. */
  opener: string;
  /** The tool's data directory, `XDG_DATA_HOME`, which the test may fill. */
  data: string;
  /** Lets slow reading's stream print its next line. */
  release: () => Promise<void>;
};

/**
 * Reads the first two lines that flightdeck prints to standard error. The
 * second line names the opener page: `Open <path> in a browser. ...`.
 */
async function openerOf(child: ChildProcess): Promise<string> {
  if (child.stderr === null) {
    throw new Error("flightdeck's standard error is not piped");
  }
  const lines: string[] = [];
  const reader = createInterface({ input: child.stderr });
  let timer: NodeJS.Timeout | undefined;
  const late = new Promise<never>((_, reject) => {
    timer = setTimeout(
      () => reject(new Error("flightdeck did not print its opener page in time")),
      startLimitMs,
    );
  });
  try {
    const read = (async () => {
      for await (const line of reader) {
        lines.push(line);
        if (lines.length === 2) {
          return line;
        }
      }
      throw new Error(`flightdeck stopped after printing: ${lines.join(" / ")}`);
    })();
    const second = await Promise.race([read, late]);
    const opener = /^Open (.+) in a browser\./.exec(second)?.[1];
    if (opener === undefined) {
      throw new Error(`flightdeck's second line names no opener page: ${second}`);
    }
    return pathToFileURL(opener).href;
  } finally {
    clearTimeout(timer);
    reader.close();
  }
}

/**
 * The checked test, with `served`: flightdeck serving the debug build's
 * page for `tool`, shelf by default, or reading, as
 * scripts/install-test-tools.sh installs them. "slow reading" is reading
 * whose `entries stream` prints each line only when the test calls
 * `release`. Each test starts its own flightdeck, because a launch code
 * works once, and stops it when the test ends. The Rust lane builds both
 * before this lane runs.
 */
export const test = checked.extend<{
  tool: "shelf" | "reading" | "slow reading";
  served: Served;
}>({
  tool: ["shelf", { option: true }],
  served: async ({ tool }, use) => {
    const home = await mkdtemp(path.join(tmpdir(), "flightdeck-acceptance-"));
    const data = path.join(home, "data");
    const installed = (name: string) => path.join(root, "target/test-tools/bin", name);
    const slow = tool === "slow reading" ? await slowReading(home, installed("reading")) : null;
    const child = spawn(
      path.join(root, "target/debug/flightdeck"),
      ["open", slow?.program ?? installed(tool), "--no-browser"],
      {
        env: {
          ...process.env,
          XDG_CONFIG_HOME: path.join(home, "config"),
          XDG_DATA_HOME: data,
          XDG_STATE_HOME: path.join(home, "state"),
          XDG_CACHE_HOME: path.join(home, "cache"),
        },
        stdio: ["ignore", "ignore", "pipe"],
      },
    );
    try {
      const release = async () => {
        if (slow === null) {
          throw new Error(`${tool} has no stream to release`);
        }
        await slow.release();
      };
      await use({ opener: await openerOf(child), data, release });
    } finally {
      await slow?.close();
      if (child.exitCode === null) {
        child.kill("SIGTERM");
        await once(child, "exit");
      }
      await rm(home, { recursive: true, force: true });
    }
  },
});

// A package registry for the install check, which serves the tarballs in
// one directory over HTTP on 127.0.0.1. It answers the two requests pnpm
// makes to install a package, its metadata and its tarball, and nothing
// else: it has no publish, no search, no login and no other package. It is
// not a general registry; the check needs only that pnpm resolves the
// @crumbkit packages by exact version, as it will from npmjs.
//
// Usage: node registry.mjs <tarball-dir> <port-file>
// It writes the port it listens on to <port-file> and runs until killed.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { join } from "node:path";

const [dir, portFile] = process.argv.slice(2);
if (dir === undefined || portFile === undefined) {
  console.error("usage: node registry.mjs <tarball-dir> <port-file>");
  process.exit(2);
}

// Every version is dated well past pnpm's minimumReleaseAge, so the age
// check passes for these packages and still applies to every other one,
// which pnpm reads from npmjs.
const PUBLISHED = "2020-01-01T00:00:00.000Z";

/** name -> { versions: version -> manifest, files: file name -> bytes } */
const packages = new Map();
const tarballs = new Map();
for (const file of readdirSync(dir).filter((name) => name.endsWith(".tgz"))) {
  const bytes = readFileSync(join(dir, file));
  const manifest = JSON.parse(
    execFileSync("tar", ["-xzOf", join(dir, file), "package/package.json"], { encoding: "utf8" }),
  );
  const entry = packages.get(manifest.name) ?? { versions: {} };
  entry.versions[manifest.version] = {
    ...manifest,
    dist: {
      tarball: `/-/${file}`,
      integrity: `sha512-${createHash("sha512").update(bytes).digest("base64")}`,
      shasum: createHash("sha1").update(bytes).digest("hex"),
    },
  };
  packages.set(manifest.name, entry);
  tarballs.set(file, bytes);
}

const server = createServer((request, response) => {
  const path = decodeURIComponent(new URL(request.url ?? "/", "http://registry").pathname);
  const origin = `http://${request.headers.host}`;
  if (path.startsWith("/-/")) {
    const bytes = tarballs.get(path.slice(3));
    if (bytes !== undefined) {
      response.writeHead(200, { "content-type": "application/octet-stream" });
      response.end(bytes);
      return;
    }
  }
  const entry = packages.get(path.slice(1));
  if (entry !== undefined) {
    const versions = Object.fromEntries(
      Object.entries(entry.versions).map(([version, manifest]) => [
        version,
        { ...manifest, dist: { ...manifest.dist, tarball: origin + manifest.dist.tarball } },
      ]),
    );
    const names = Object.keys(versions);
    const body = {
      name: path.slice(1),
      "dist-tags": { latest: names[names.length - 1] },
      versions,
      time: {
        created: PUBLISHED,
        modified: PUBLISHED,
        ...Object.fromEntries(names.map((version) => [version, PUBLISHED])),
      },
      modified: PUBLISHED,
    };
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify(body));
    return;
  }
  response.writeHead(404, { "content-type": "application/json" });
  response.end(JSON.stringify({ error: "not_found" }));
});

server.listen(0, "127.0.0.1", () => {
  const address = server.address();
  if (address === null || typeof address === "string") throw new Error("no port");
  writeFileSync(portFile, String(address.port));
});

// Checks a packed package.json: its name and version, that it is public,
// that every export points into dist/, and that every dependency is an
// exact version. pnpm refuses a git, tarball or file URL in a dependency's
// manifest by default (blockExoticSubdeps), and workspace: means nothing
// outside this repository.
//
// Usage: node manifest.mjs <package.json> <name> <version>
// It prints each problem and exits 1 when there is one.
import { readFileSync } from "node:fs";

const [file, name, version] = process.argv.slice(2);
const manifest = JSON.parse(readFileSync(file, "utf8"));
const problems = [];

if (manifest.name !== name) problems.push(`name is ${manifest.name}, not ${name}`);
if (manifest.version !== version) problems.push(`version is ${manifest.version}, not ${version}`);
if (manifest.private === true) problems.push("it is private");

const exact = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;
for (const field of ["dependencies", "optionalDependencies"]) {
  for (const [dependency, spec] of Object.entries(manifest[field] ?? {})) {
    if (!exact.test(spec)) problems.push(`${field}.${dependency} is ${spec}, not an exact version`);
  }
}
// A peer is a range the adopter's own copy must meet, so it is never a URL.
const url = /:|\//;
for (const [dependency, spec] of Object.entries(manifest.peerDependencies ?? {})) {
  if (url.test(spec)) problems.push(`peerDependencies.${dependency} is ${spec}, a URL`);
}

const targets = [];
const walk = (value) => {
  if (typeof value === "string") targets.push(value);
  else if (value !== null && typeof value === "object") Object.values(value).forEach(walk);
};
walk(manifest.exports);
if (targets.length === 0) problems.push("it has no exports");
for (const target of targets) {
  if (!target.startsWith("./dist/")) problems.push(`an export points at ${target}, outside dist/`);
}

for (const problem of problems) console.error(`${file}: ${problem}`);
process.exit(problems.length === 0 ? 0 : 1);

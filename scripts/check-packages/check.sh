#!/usr/bin/env bash
# Checks the two package tarballs that scripts/pack-packages/pack.sh wrote,
# the way an adopting front end meets them.
#
# Usage: scripts/check-packages/check.sh <version> <dir>
#
# <dir> holds crumbkit-flightdeck-headless-<version>.tgz and
# crumbkit-flightdeck-react-<version>.tgz. ui/ must be installed, because
# the check packs @crumbkit/ui from the tag that ui/pnpm-lock.yaml pins.
#
# 1. Each tarball holds dist/ and no src/, carries <version> and its
#    @crumbkit/flightdeck-* name, and names every dependency by an exact
#    version, so pnpm's default blockExoticSubdeps accepts it.
# 2. A project in a temporary directory, with pnpm's default settings,
#    minimumReleaseAge: 10080 and exact react, react-dom and typescript pins,
#    installs both with one `pnpm add`. registry.mjs serves the @crumbkit
#    scope from the two tarballs and the tag's pack of @crumbkit/ui, and
#    every other package comes from npmjs. No override is set, so a refused
#    dependency fails the install.
# 3. node_modules/.pnpm holds exactly one @crumbkit/ui.
# 4. consumer.ts type-checks with tsc --noEmit, and under Node it renders the
#    sample compliance.json's headline and tile counts, which must match
#    expected.html.
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <version> <dir>" >&2
  exit 2
fi
version=$1
dir=$(cd "$2" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
ui=$root/ui

fail() {
  echo "check-packages: $*" >&2
  exit 1
}

work=$(mktemp -d)
registry_pid=
cleanup() {
  if [ -n "$registry_pid" ]; then
    kill "$registry_pid" 2> /dev/null || true
    wait "$registry_pid" 2> /dev/null || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT

# 1. The tarballs themselves.
for package in headless react; do
  tarball=$dir/crumbkit-flightdeck-$package-$version.tgz
  [ -f "$tarball" ] || fail "$tarball is missing"
  files=$(tar -tzf "$tarball")
  grep -q '^package/dist/.*\.js$' <<< "$files" || fail "$tarball holds no dist/*.js"
  grep -q '^package/dist/.*\.d\.ts$' <<< "$files" || fail "$tarball holds no dist/*.d.ts"
  if grep -q '^package/src/' <<< "$files"; then fail "$tarball holds src/"; fi
  tar -xzOf "$tarball" package/package.json > "$work/$package.json"
  node "$here/manifest.mjs" "$work/$package.json" "@crumbkit/flightdeck-$package" "$version" ||
    fail "$tarball's package.json is not one an adopter can install"
done

# 2. The registry, seeded with the two tarballs and the tag's @crumbkit/ui.
mkdir "$work/registry" "$work/consumer"
cp "$dir/crumbkit-flightdeck-headless-$version.tgz" "$dir/crumbkit-flightdeck-react-$version.tgz" "$work/registry/"
installed_ui=$ui/headless/node_modules/@crumbkit/ui
[ -f "$installed_ui/package.json" ] || fail "ui/ is not installed; run pnpm install --frozen-lockfile in ui/"
npm pack --silent --ignore-scripts --pack-destination "$work/registry" "$installed_ui" > /dev/null
node "$here/registry.mjs" "$work/registry" "$work/port" &
registry_pid=$!
for _ in $(seq 50); do
  [ -s "$work/port" ] && break
  sleep 0.1
done
[ -s "$work/port" ] || fail "the local registry did not start"
port=$(cat "$work/port")

# The consumer pins the versions the page itself is built with. Node reads
# them before the consumer's own package.json exists.
pins=$(node -e '
  const ui = require(process.argv[1]);
  const pins = {
    dependencies: { react: ui.dependencies.react, "react-dom": ui.dependencies["react-dom"] },
    devDependencies: Object.fromEntries(
      ["@types/node", "@types/react", "@types/react-dom", "typescript"].map((name) => [name, ui.devDependencies[name]]),
    ),
  };
  console.log(JSON.stringify({ name: "consumer", private: true, type: "module", packageManager: ui.packageManager, ...pins }, null, 2));
' "$ui/package.json")
cd "$work/consumer"
printf '%s\n' "$pins" > package.json
cat > pnpm-workspace.yaml << 'EOF'
minimumReleaseAge: 10080
EOF
cat > .npmrc << EOF
@crumbkit:registry=http://127.0.0.1:$port/
EOF
pnpm add "@crumbkit/flightdeck-headless@$version" "@crumbkit/flightdeck-react@$version" ||
  fail "pnpm could not install the tarballs with its default settings"

# 3. One copy of @crumbkit/ui.
copies=$(find node_modules/.pnpm -maxdepth 1 -name '@crumbkit+ui@*' | wc -l | tr -d ' ')
[ "$copies" = 1 ] || fail "node_modules/.pnpm holds $copies copies of @crumbkit/ui, and an adopter needs one"

# 4. The consumer type-checks and renders.
cp "$here/consumer.ts" "$here/tsconfig.json" .
pnpm exec tsc --noEmit -p tsconfig.json || fail "consumer.ts does not type-check against the tarballs"
node consumer.ts "$root/tests/fixtures/compliance/compliance.json" > rendered.html ||
  fail "consumer.ts did not render"
diff -u "$here/expected.html" rendered.html || fail "the rendered page differs from expected.html"

echo "check-packages: both tarballs install, type-check and render with pnpm's default settings"

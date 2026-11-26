#!/usr/bin/env bash
# Builds and packs @crumbkit/flightdeck-headless and @crumbkit/flightdeck-react
# at a release's version.
#
# Usage: scripts/pack-packages/pack.sh <version> <out-dir> [--registry require|warn]
#
# It writes <out-dir>/crumbkit-flightdeck-headless-<version>.tgz and
# <out-dir>/crumbkit-flightdeck-react-<version>.tgz. ui/ must be installed
# with `pnpm install --frozen-lockfile`.
#
# The committed manifests stay at 0.0.0 and name @crumbkit/ui by its git
# tag, which an adopter's pnpm refuses in a dependency's manifest. For the
# pack, the script sets both versions to <version> and names @crumbkit/ui by
# the exact version that the pinned tag's package.json holds; pnpm pack
# names the core by its version in place of workspace:* and takes the
# exports from publishConfig. It puts the committed manifests back when it
# exits.
#
# A tarball that names @crumbkit/ui by version installs only once npmjs
# holds that version, with the same dist/ as the tag. The script checks
# both. With --registry require, the setting of a release that publishes, a
# missing version or a different dist/ stops the pack; with --registry warn,
# the default and the setting of every other run, it prints a warning and
# packs.
set -euo pipefail

usage() {
  echo "usage: $0 <version> <out-dir> [--registry require|warn]" >&2
  exit 2
}
[ "$#" -ge 2 ] || usage
version=$1
out=$2
registry=warn
if [ "$#" -eq 4 ] && [ "$3" = --registry ]; then
  registry=$4
elif [ "$#" -ne 2 ]; then
  usage
fi
case "$registry" in require | warn) ;; *) usage ;; esac
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "pack-packages: the version is $version, and a release version looks like 0.6.0" >&2
  exit 2
fi

fail() {
  echo "pack-packages: $*" >&2
  exit 1
}

mkdir -p "$out"
out=$(cd "$out" && pwd)
root=$(cd "$(dirname "$0")/../.." && pwd)
ui=$root/ui
cd "$ui"

# The pin both manifests share, and the version the tag's package.json holds.
pin=$(node -p "require('./headless/package.json').dependencies['@crumbkit/ui']")
react_pin=$(node -p "require('./react/package.json').dependencies['@crumbkit/ui']")
[ "$pin" = "$react_pin" ] || fail "headless names @crumbkit/ui $pin and react names $react_pin"
installed=headless/node_modules/@crumbkit/ui
[ -f "$installed/package.json" ] || fail "ui/ is not installed; run pnpm install --frozen-lockfile in ui/"
ui_version=$(node -p "require('./$installed/package.json').version")

# npmjs must hold that version, built from the same dist/.
work=$(mktemp -d)
backup=$work/manifests
mkdir "$backup"
cp headless/package.json "$backup/headless.json"
cp react/package.json "$backup/react.json"
restore() {
  cp "$backup/headless.json" "$ui/headless/package.json"
  cp "$backup/react.json" "$ui/react/package.json"
  rm -rf "$work"
}
trap restore EXIT

registry_problem() {
  if [ "$registry" = require ]; then
    fail "$1"
  fi
  echo "::warning::pack-packages: $1; a release that publishes would stop here"
}
if ! (cd "$work" && npm pack --silent --ignore-scripts "@crumbkit/ui@$ui_version" > "$work/ui-pack" 2> "$work/ui-error"); then
  registry_problem "npmjs holds no @crumbkit/ui $ui_version, which $pin pins"
else
  mkdir "$work/ui"
  tar -xzf "$work/$(cat "$work/ui-pack")" -C "$work/ui"
  if ! diff -r "$work/ui/package/dist" "$installed/dist" > "$work/ui-diff"; then
    cat "$work/ui-diff" >&2
    registry_problem "@crumbkit/ui $ui_version on npmjs has a dist/ that differs from $pin's"
  fi
fi

# The build runs on the committed manifests, which match the lockfile, so
# pnpm finds nothing to install before the script. The core builds first,
# because the hooks' build reads its declarations. Then the release's
# manifests and the pack.
for package in headless react; do
  (cd "$package" && pnpm run build)
done
for package in headless react; do
  (
    cd "$package"
    npm pkg set "version=$version" "dependencies.@crumbkit/ui=$ui_version"
    pnpm pack --pack-destination "$out" > /dev/null
  )
  [ -f "$out/crumbkit-flightdeck-$package-$version.tgz" ] ||
    fail "pnpm pack wrote no crumbkit-flightdeck-$package-$version.tgz"
done
echo "pack-packages: packed $version into $out, naming @crumbkit/ui $ui_version"

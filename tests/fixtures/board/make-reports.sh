#!/bin/sh
# Writes the rules/report/v1 documents under tests/fixtures/compliance/reports
# from a real `board rules report`, and the copy of the report's schema at
# tests/fixtures/board/rules-report-v1.schema.json, so the compliance build's tests read what Board
# prints. Run it with the Board that scripts/install-test-tools.sh pins:
#
#   tests/fixtures/board/make-reports.sh target/test-tools/bin/board
#
# It builds a throwaway controls board in a temporary directory. The board's
# origin is owner/controls, and it imposes these rules on every mounted
# repository:
#
# - /rules/rust/file-size, a budget of 60 bytes per Rust file;
# - /rules/toolchains/rust, a version rule of at least 1.99;
# - /rules/rust/server, a written rule in both the rust and server groups;
# - /rules/payments/v1-signer, a zero rule on /suites/signer-callers;
# - /rules/go/vet, a Go rule that reaches no file;
# - /rules/writing/plain, a judged rule whose rubric is missing, which Board
#   reports as unreadable.
#
# It mounts three repositories, each of two commits:
#
# - owner/app meets every measured rule, holds its own copy of
#   /rules/rust/server with another requirement and one extra rule, and its
#   compliance suite passed at its first commit;
# - owner/api has a Rust file over the budget, pins Rust 1.90, and its
#   compliance suite failed a case;
# - owner/ci is reported as GitHub Actions run 7 would report it.
#
# Board writes each report's `generated` time when the script runs, so the
# tests pass the build a `--built-at` relative to it.
set -eu

BOARD=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/../compliance/reports"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

export HOME="$WORK/home"
export XDG_CONFIG_HOME="$WORK/xdg/config" XDG_DATA_HOME="$WORK/xdg/data"
export XDG_STATE_HOME="$WORK/xdg/state" XDG_CACHE_HOME="$WORK/xdg/cache"
export GIT_AUTHOR_NAME=fixture GIT_AUTHOR_EMAIL=fixture@example.invalid
export GIT_COMMITTER_NAME=fixture GIT_COMMITTER_EMAIL=fixture@example.invalid
export GIT_AUTHOR_DATE=2026-11-20T09:00:00Z GIT_COMMITTER_DATE=2026-11-20T09:00:00Z
mkdir -p "$HOME"

# repository NAME FILE=TEXT...: a repository of two commits, the first with
# the files given and the second adding NOTES.md. Prints nothing.
repository() {
  dir="$WORK/repos/$1"
  shift
  git init -q -b main "$dir"
  if [ "${WITH_TYPES:-}" = yes ]; then
    mkdir -p "$dir/.board/types/rules"
    cp -R "$TYPES" "$dir/.board/types/rules/rule"
  fi
  for pair in "$@"; do
    file=${pair%%=*}
    mkdir -p "$dir/$(dirname "$file")"
    printf '%s\n' "${pair#*=}" > "$dir/$file"
  done
  git -C "$dir" add -A
  git -C "$dir" commit -q -m first
  printf 'notes\n' > "$dir/NOTES.md"
  git -C "$dir" add -A
  git -C "$dir" commit -q -m second
}

# first NAME: the first commit of repository NAME.
first() { git -C "$WORK/repos/$1" rev-list --max-parents=0 HEAD; }
# head NAME: the last commit of repository NAME.
head_of() { git -C "$WORK/repos/$1" rev-parse HEAD; }

CONTROLS="$WORK/controls"
mkdir -p "$CONTROLS/.board"
git init -q -b main "$CONTROLS"
git -C "$CONTROLS" remote add origin https://github.com/owner/controls.git
printf '%s\n' '{"$type": "board/config/v1", "commit_on_put": false}' > "$CONTROLS/.board/board.json"
cd "$CONTROLS"
"$BOARD" plugin enable rules > /dev/null
"$BOARD" plugin enable runs > /dev/null

# The repository's own rules need the rule schema in its own board.
TYPES="$CONTROLS/.board/types/rules/rule"

own_rule='{"$type": "rules/rule/v1", "requirement": "Name server handlers after their route.", "purpose": "A reader finds the handler of a route.", "paths": ["src/*"]}'
extra_rule='{"$type": "rules/rule/v1", "requirement": "Keep the app'"'"'s handlers in src.", "purpose": "Only this repository wants it.", "paths": ["src/*"]}'
mkdir -p "$WORK/repos"
WITH_TYPES=yes repository app \
  'src/main.rs=fn main() {}' \
  'rust-toolchain.toml=channel = "1.100.0"' \
  '.board/board.json={"$type": "board/config/v1", "plugins": ["rules"]}' \
  "rules/rust/server.json=$own_rule" \
  "rules/app/handlers.json=$extra_rule"
repository api \
  'src/main.rs=fn main() { let handlers = serve_every_route(); route(handlers); }' \
  'rust-toolchain.toml=channel = "1.90.0"'
repository ci \
  'src/main.rs=fn main() {}' \
  'rust-toolchain.toml=channel = "1.100.0"'

# write PATH TEXT: writes TEXT to PATH in the controls board.
write() {
  mkdir -p "$(dirname "$CONTROLS/$1")"
  printf '%s\n' "$2" > "$CONTROLS/$1"
}

write rules/rust/file-size.json '{"$type": "rules/rule/v1", "requirement": "Keep each Rust file under 60 bytes.", "purpose": "A reader takes in a small file whole.", "paths": ["repos/*/src/*.rs"], "checker": {"check": "budget", "unit": "bytes", "per": "file", "max": 60}}'
write rules/toolchains/rust.json '{"$type": "rules/rule/v1", "requirement": "Pin Rust 1.99 or later.", "purpose": "Every repository builds on one toolchain.", "paths": ["repos/*/rust-toolchain.toml"], "checker": {"check": "version", "at_least": "1.99", "capture": "channel = \"([0-9.]+)\""}}'
write rules/rust/server.json '{"$type": "rules/rule/v1", "requirement": "Name each server handler after its route.", "purpose": "A reader finds the handler of a route.", "paths": ["repos/*/src/*"]}'
write rules/payments/v1-signer.json '{"$type": "rules/rule/v1", "requirement": "No service still calls the v1 webhook signer.", "purpose": "The v1 signer can be removed.", "paths": ["repos/*/src/*"], "checker": {"check": "zero", "suite": "/suites/signer-callers", "measures": ["remaining"], "within_days": 7}}'
write rules/go/vet.json '{"$type": "rules/rule/v1", "requirement": "Vet every Go package.", "purpose": "Vet finds mistakes the compiler allows.", "paths": ["repos/*/*.go"]}'
write rules/writing/plain.json '{"$type": "rules/rule/v1", "requirement": "Write the README plainly.", "purpose": "Readers skim it.", "paths": ["repos/*/src/*"], "checker": {"check": "judged", "rubric": "/rubrics/missing.md"}}'
write rules/groups/rust.json '{"$type": "rules/group/v1", "title": "Rust", "purpose": "The Rust rules.", "members": ["/rules/rust/*", "/rules/toolchains/rust"]}'
write rules/groups/server.json '{"$type": "rules/group/v1", "title": "Server", "purpose": "The rules of every server.", "members": ["/rules/rust/server", "/rules/payments/*"]}'

write suites/signer-callers.json '{"$type": "runs/suite/v1", "subjects": ["owner/app", "owner/api", "owner/ci"], "method": "Count the callers of the v1 signer."}'
write suites/signer-callers/runs/20261124T1000Z.json '{"$type": "runs/run/v1", "suite": "/suites/signer-callers", "started": "2026-11-24T10:00:00Z", "finished": "2026-11-24T10:05:00Z", "scope": "full", "dirty": false, "measures": {"remaining": 2}, "results": []}'
write suites/app-rust-standard.json '{"$type": "runs/suite/v1", "subjects": ["owner/app"], "kind": "compliance", "method": "Audit the repository against the Rust standard.", "cases": [{"id": "RUST-1"}, {"id": "RUST-2"}]}'
write suites/app-rust-standard/runs/20261121T0900Z.json "{\"\$type\": \"runs/run/v1\", \"suite\": \"/suites/app-rust-standard\", \"started\": \"2026-11-21T09:00:00Z\", \"finished\": \"2026-11-21T09:30:00Z\", \"scope\": \"full\", \"dirty\": false, \"versions\": {\"commit\": \"$(first app)\"}, \"results\": [{\"case\": \"RUST-1\", \"outcome\": \"pass\"}, {\"case\": \"RUST-2\", \"outcome\": \"pass\"}]}"
write suites/api-rust-standard.json '{"$type": "runs/suite/v1", "subjects": ["owner/api"], "kind": "compliance", "method": "Audit the repository against the Rust standard."}'
write suites/api-rust-standard/runs/20261124T0900Z.json "{\"\$type\": \"runs/run/v1\", \"suite\": \"/suites/api-rust-standard\", \"started\": \"2026-11-24T09:00:00Z\", \"finished\": \"2026-11-24T09:30:00Z\", \"scope\": \"full\", \"dirty\": false, \"versions\": {\"commit\": \"$(first api)\"}, \"results\": [{\"case\": \"RUST-1\", \"outcome\": \"pass\"}, {\"case\": \"RUST-7\", \"outcome\": \"fail\", \"note\": \"src/main.rs holds two handlers.\"}]}"

for name in app api ci; do
  write "repos/$name.json" "{\"\$type\": \"board/mount/v1\", \"repo\": \"$WORK/repos/$name\", \"follow\": \"main\", \"commit\": \"$(head_of $name)\"}"
done
git -C "$CONTROLS" add -A
git -C "$CONTROLS" commit -q -m controls

mkdir -p "$OUT/owner"
"$BOARD" rules report --path /repos/app --repository owner/app > "$OUT/owner/app.json"
"$BOARD" rules report --path /repos/api --repository owner/api > "$OUT/owner/api.json"
GITHUB_REPOSITORY=owner/ci GITHUB_RUN_ID=9001 GITHUB_RUN_NUMBER=7 GITHUB_SHA=$(head_of ci) \
  "$BOARD" rules report --path /repos/ci --repository owner/ci > "$OUT/owner/ci.json"
# The schema that `board plugin enable rules` wrote into the controls
# board. This Board's `board schemas show` serves only its own command
# documents, so tests/compliance.rs reads the schema the same way.
cp "$CONTROLS/.board/types/rules/report/v1.schema.json" "$HERE/rules-report-v1.schema.json"

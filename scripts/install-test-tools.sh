#!/bin/sh
# Installs shelf and reading, crumbkit's example tools, under
# target/test-tools/bin, where flightdeck's tests find them. The commit is
# the crumbkit v0.3.2 release, the first whose shelf records the program
# that started it, takes `--exclude-program` on its usage reports and marks
# crumbkit's own commands `built_in` in `commands list`.
# shelf and reading are not published, so the script builds them from that
# commit. It moves forward when flightdeck starts to read a document that a
# later crumbkit prints, and tests/fixtures/crumbkit/ moves with it.
set -eu

CRUMBKIT_REV=8ee2eadd29eba89a9d65033b38cc861701be21d5

cd "$(dirname "$0")/.."
cargo install --locked \
  --git https://github.com/crumbkit/crumbkit \
  --rev "$CRUMBKIT_REV" \
  --root target/test-tools \
  shelf reading

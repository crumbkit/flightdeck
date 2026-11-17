#!/bin/sh
# Installs shelf and reading, crumbkit's example tools, under
# target/test-tools/bin, where flightdeck's tests find them. The commit is
# crumbkit/crumbkit#393, the first whose `commands list` says which command
# runs a program (`effects.runs_program`) and which argument takes its values
# after `--` (`after_double_dash`). It also records the program that started
# shelf, takes `--exclude-program` on its usage reports and marks crumbkit's
# own commands `built_in`.
# shelf and reading are not published, so the script builds them from that
# commit. It moves forward when flightdeck starts to read a document that a
# later crumbkit prints, and tests/fixtures/crumbkit/ moves with it.
set -eu

CRUMBKIT_REV=cb73bd8ea968033c2119ec91534311234018584e

cd "$(dirname "$0")/.."
cargo install --locked \
  --git https://github.com/crumbkit/crumbkit \
  --rev "$CRUMBKIT_REV" \
  --root target/test-tools \
  shelf reading

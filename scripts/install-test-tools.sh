#!/bin/sh
# Installs shelf and reading, crumbkit's example tools, under
# target/test-tools/bin, where flightdeck's tests find them. The commit is the
# crumbkit v0.6.0 release, the one flightdeck builds on. Its `commands list`
# says which command runs a program (`effects.runs_program`) and which
# argument takes its values after `--` (`after_double_dash`), shelf records
# the program that started it, takes `--exclude-program` on its usage reports
# and marks crumbkit's own commands `built_in`.
# shelf and reading are not published, so the script builds them from that
# commit. It moves forward when flightdeck starts to read a document that a
# later crumbkit prints, and tests/fixtures/crumbkit/ moves with it.
set -eu

# The commit that the tag v0.6.0 names.
CRUMBKIT_REV=de1bc6b8aa94fddf3aca6a30783f1ff002e5cfdc

cd "$(dirname "$0")/.."
cargo install --locked \
  --git https://github.com/crumbkit/crumbkit \
  --rev "$CRUMBKIT_REV" \
  --root target/test-tools \
  shelf reading

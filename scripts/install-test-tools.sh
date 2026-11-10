#!/bin/sh
# Installs shelf and reading, crumbkit's example tools, under
# target/test-tools/bin, where flightdeck's tests find them. The commit is
# crumbkit/crumbkit#328, the first whose shelf mounts `self runs list` and
# reports the median figures that flightdeck reads.
# shelf and reading are not published, so the script builds them from that
# commit. It moves forward when flightdeck starts to read a document that a
# later crumbkit prints, and tests/fixtures/crumbkit/ moves with it.
set -eu

CRUMBKIT_REV=644f776e29ca7332202653830518e7f41b7d319f

cd "$(dirname "$0")/.."
cargo install --locked \
  --git https://github.com/crumbkit/crumbkit \
  --rev "$CRUMBKIT_REV" \
  --root target/test-tools \
  shelf reading

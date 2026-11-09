#!/bin/sh
# Installs shelf and reading, crumbkit's example tools, under
# target/test-tools/bin, where flightdeck's tests find them. The commit is the
# commit of the crumbkit 0.2.0 release, the version that flightdeck builds on.
# shelf and reading are not published, so the script builds them from that
# commit. It moves forward when flightdeck starts to read a document that a
# later crumbkit prints, and tests/fixtures/crumbkit/ moves with it.
set -eu

CRUMBKIT_REV=45d0015e1f94f6bcf26887f943aa14bce8fdb510

cd "$(dirname "$0")/.."
cargo install --locked \
  --git https://github.com/crumbkit/crumbkit \
  --rev "$CRUMBKIT_REV" \
  --root target/test-tools \
  shelf reading

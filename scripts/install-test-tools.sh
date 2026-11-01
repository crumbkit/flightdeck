#!/bin/sh
# Installs shelf and reading, crumbkit's example tools, under
# target/test-tools/bin, where flightdeck's tests find them. The commit is the
# crumbkit 0.1.1 release. It moves forward when flightdeck starts to read a
# document that a later crumbkit prints, and tests/fixtures/crumbkit/ moves
# with it.
set -eu

CRUMBKIT_REV=a85fa49c767a4313bbfa3261b84c4f6b4cbc73c1

cd "$(dirname "$0")/.."
cargo install --locked \
  --git https://github.com/crumbkit/crumbkit \
  --rev "$CRUMBKIT_REV" \
  --root target/test-tools \
  shelf reading

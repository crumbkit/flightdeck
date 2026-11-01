#!/bin/sh
# Installs shelf and reading, crumbkit's example tools, under
# target/test-tools/bin, where flightdeck's tests find them. The commit is the
# crumbkit 0.1.0 release. It moves forward when flightdeck starts to read a
# document that a later crumbkit prints, and tests/fixtures/crumbkit/ moves
# with it.
set -eu

CRUMBKIT_REV=31245d63b750013a45fd8adcb23ac87ac09a4f1d

cd "$(dirname "$0")/.."
cargo install --locked \
  --git https://github.com/crumbkit/crumbkit \
  --rev "$CRUMBKIT_REV" \
  --root target/test-tools \
  shelf reading

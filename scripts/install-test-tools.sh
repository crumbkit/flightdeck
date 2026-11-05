#!/bin/sh
# Installs shelf and reading, crumbkit's example tools, under
# target/test-tools/bin, where flightdeck's tests find them. The commit is the
# merge of crumbkit/crumbkit#291, after the crumbkit 0.1.1 release, whose
# `commands list` marks a command that prints no result. It moves forward when
# flightdeck starts to read a document that a later crumbkit prints, and
# tests/fixtures/crumbkit/ moves with it.
set -eu

CRUMBKIT_REV=b7f02c45fe90ac461e90f59a2c559ce11bba479c

cd "$(dirname "$0")/.."
cargo install --locked \
  --git https://github.com/crumbkit/crumbkit \
  --rev "$CRUMBKIT_REV" \
  --root target/test-tools \
  shelf reading

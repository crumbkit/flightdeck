#!/bin/sh
# Installs shelf and reading, crumbkit's example tools, and a pinned Board
# under target/test-tools/bin, where flightdeck's tests find them. The commit is the
# crumbkit v0.6.0 release, the one flightdeck builds on. Its `commands list`
# says which command runs a program (`effects.runs_program`) and which
# argument takes its values after `--` (`after_double_dash`), shelf records
# the program that started it, takes `--exclude-program` on its usage reports
# and marks crumbkit's own commands `built_in`.
# shelf and reading are not published, so the script builds them from that
# commit. It moves forward when flightdeck starts to read a document that a
# later crumbkit prints, and tests/fixtures/crumbkit/ moves with it.
# Board is pinned for the contract test in tests/compliance.rs, which
# compares the copy of Board's rules/report/v1 schema in tests/fixtures/board/
# with the one this Board installs. The commit is the head of
# crumbkit/board#358, which adds `board rules report`; it moves to the merge
# commit once that pull request lands, and tests/fixtures/board/make-reports.sh
# rewrites the fixtures with it. Board builds for macOS and Linux, so Windows
# skips it, and the contract test runs on Unix alone.
set -eu

# The commit that the tag v0.6.0 names.
CRUMBKIT_REV=de1bc6b8aa94fddf3aca6a30783f1ff002e5cfdc
# The head of crumbkit/board#358.
BOARD_REV=9ef744d362cf1cb48b682c68b8fb8d85f0686c8d

cd "$(dirname "$0")/.."
cargo install --locked \
  --git https://github.com/crumbkit/crumbkit \
  --rev "$CRUMBKIT_REV" \
  --root target/test-tools \
  shelf reading
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) ;;
  *)
    cargo install --locked \
      --git https://github.com/crumbkit/board \
      --rev "$BOARD_REV" \
      --root target/test-tools \
      board-cli
    ;;
esac

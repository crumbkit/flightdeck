#!/bin/sh
# Fails when a commit in BASE..HEAD matches the extended regular expression
# in the COMMIT_PATTERN variable, case-insensitively, in its message, its
# diff, its author email or its committer email. CI sets the variable from
# the repository secret of the same name. The script never prints the
# pattern or the text that matched, only the commit and the part that
# matched. When the variable is empty, as on a pull request from a fork, it
# prints a notice and passes.
#
# Usage: commit-check.sh BASE HEAD
# A BASE of forty zeros, which GitHub sends for a new branch, checks every
# commit that HEAD reaches. So does a BASE that the checkout lacks, such as
# the old tip of a branch after a force push, and the script says so.
#
# grep reads the pattern from a here-document on file descriptor 3, so the
# pattern never appears in a process's argument list.
set -eu

if [ "$#" -ne 2 ]; then
  echo "usage: commit-check.sh BASE HEAD" >&2
  exit 2
fi
base=$1
head=$2

if [ -z "${COMMIT_PATTERN:-}" ]; then
  echo "::notice::COMMIT_PATTERN is empty, so the commit check is skipped"
  exit 0
fi

# Reads standard input and succeeds when it matches the pattern.
matches() {
  grep -E -i -q -f /dev/fd/3 3<<PATTERN
$COMMIT_PATTERN
PATTERN
}

# grep exits with 2 on a pattern it cannot read, which would otherwise pass
# every commit as one that does not match.
status=0
printf 'x\n' | matches || status=$?
if [ "$status" -gt 1 ]; then
  echo "::error::COMMIT_PATTERN is not an extended regular expression that grep reads"
  exit 2
fi

if [ "$base" = "0000000000000000000000000000000000000000" ]; then
  commits=$(git rev-list "$head")
elif ! git cat-file -e "$base^{commit}" 2>/dev/null; then
  echo "::notice::the base commit $base is not in the checkout, as after a force push, so every commit that $head reaches is checked"
  commits=$(git rev-list "$head")
else
  commits=$(git rev-list "$base..$head")
fi

failed=0
for commit in $commits; do
  if git log -1 --format=%B "$commit" | matches; then
    echo "::error::commit $commit matches the commit pattern in its message"
    failed=1
  fi
  if git show --format= --no-color --no-ext-diff "$commit" | matches; then
    echo "::error::commit $commit matches the commit pattern in its diff"
    failed=1
  fi
  if git log -1 --format=%ae "$commit" | matches; then
    echo "::error::commit $commit matches the commit pattern in its author email"
    failed=1
  fi
  if git log -1 --format=%ce "$commit" | matches; then
    echo "::error::commit $commit matches the commit pattern in its committer email"
    failed=1
  fi
done
exit "$failed"

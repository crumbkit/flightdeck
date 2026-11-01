# The runner's fake tool

`unruly` is a shell script that stands in for a crumbkit tool in
`tests/runner.rs`. It prints `unruly-commands-list.json` for
`commands list`, an empty `schemas list`, and one command for each way a
real tool can misbehave: one that panics, one that ignores SIGINT and
SIGTERM, one that prints a line longer than the test's limit, one that
prints its `TRACEPARENT`, and one that takes a value from a fixed list.

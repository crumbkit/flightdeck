# The runner's fake tool

`unruly` is a shell script that stands in for a crumbkit tool in
`tests/runner.rs`. It prints `unruly-commands-list.json` for
`commands list`, an empty `schemas list`, and one command for each way a
real tool can misbehave or a test needs:

- `child panic` exits with 101 and prints a panic on standard error.
- `child hang` prints two lines and ignores SIGINT and SIGTERM.
- `child flood` prints a line longer than the test's limit.
- `child trace` prints its `TRACEPARENT`.
- `child pick` takes a value from a fixed list.
- `child orphan` leaves a process in a session of its own that holds
  standard output open for ten seconds. It needs `perl`.
- `child chatter` prints more lines than the runner's queue holds, then
  ignores SIGINT and SIGTERM.
- `child nap` prints a document after a fifth of a second.
- `child wipe` is destructive and takes `--dry-run`. Its preview of the
  name `bad` fails without a document, and its preview of the name `slow`
  prints a document and exits with 0 on SIGINT.
- `child both` declares itself read-only and destructive at once.
- `child pair` takes two optional positional values.
- `child linger` prints its process id, then ignores SIGINT and SIGTERM
  for the number of seconds that its one value gives, and then prints a
  document.
- `child adopt` moves into the process group of the program that started
  it, then ignores SIGINT and SIGTERM. It needs `perl`.
- `child interrupt` prints a document, and on SIGINT prints 400 more and
  then a line that is no document.

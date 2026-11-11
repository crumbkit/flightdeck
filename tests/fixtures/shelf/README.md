# shelf's documents

These files are documents from shelf at the commit that
`scripts/install-test-tools.sh` pins, for the unit tests under `src/`, which
do not run shelf. They move with that commit.

- `commands-list.json` is `shelf commands list --json --full --all`, with
  `goals` on the root row: the goal headings of shelf's root help in help
  order.
- `commands-list-older.json` is the same document from shelf at crumbkit
  commit `644f776e29ca7332202653830518e7f41b7d319f` (crumbkit/crumbkit#328),
  whose rows have no `built_in` and whose reports of the usage log take no
  `--exclude-program`. The tests of a tool on an earlier crumbkit read it,
  and it stays when the commit moves.
- `books-list-with-run-step.json` is `shelf books list --json`, whose `next`
  holds one step that calls a shelf command. A second step was added by
  hand, because shelf prints no step that runs another program: it runs
  `rm` and then calls a shelf command.
- `self-figures-list.json` is `shelf self figures list --json --full --all`,
  `self-usage.json` is `shelf self usage --json --full` and
  `self-runs-list.json` is `shelf self runs list --json --full --all`, each
  run in that order after
  `books list`, two runs of `books show 99`, which fail, `books show
  --help`, two runs of `books show 2` and `books show --bogus`, in one
  session with an empty usage log.

`books-list-with-run-step.json` and the three documents of the usage log
come from shelf at crumbkit commit
`644f776e29ca7332202653830518e7f41b7d319f`. shelf at the v0.3.2 release
prints the same fields in them when no program is named, so they did not
move with it.

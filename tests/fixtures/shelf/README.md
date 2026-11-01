# shelf's documents

These files are documents from shelf at the commit that
`scripts/install-test-tools.sh` pins, for the unit tests under `src/`, which
do not run shelf. They move with that commit.

- `commands-list.json` is `shelf commands list --json --full --all`, with
  `goals` on the root row: the goal headings of shelf's root help in help
  order.
- `books-list-with-run-step.json` is `shelf books list --json`, whose `next`
  holds one step that calls a shelf command. A second step was added by
  hand, because shelf prints no step that runs another program: it runs
  `rm` and then calls a shelf command.

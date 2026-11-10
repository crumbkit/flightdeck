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
- `self-figures-list.json` is `shelf self figures list --json --full --all`
  and `self-usage.json` is `shelf self usage --json --full`, each run after
  `books list`, two runs of `books show 99`, which fail, `books show
  --help`, two runs of `books show 2` and `books show --bogus`, in one
  session with an empty usage log.

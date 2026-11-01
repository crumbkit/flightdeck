# Documents of a newer crumbkit

crumbkit 0.1.0 prints neither of the two optional fields that flightdeck
reads. These files are real documents from shelf at the commit that
`scripts/install-test-tools.sh` pins, with one field added by hand:

- `commands-list-with-goals.json` is `shelf commands list --json --full
  --all` with `goals` on the root row, the goal headings of shelf's root
  help in help order.
- `books-list-with-next.json` is `shelf books list --json` with `next`, the
  listing's next steps: one that calls a shelf command, and one that runs
  another program and then calls a shelf command.

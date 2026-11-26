# A shelf that mounts `self environment list`

`shelf` here is a shell script that stands in for a crumbkit tool that
declares its environment with starmap and mounts crumbkit-starmap's
`self environment list`. flightdeck does not depend on crumbkit-starmap,
which needs a later crumbkit, so no real tool of the test tools mounts it.

The script runs the shelf that `scripts/install-test-tools.sh` installs and
passes every command through, except three:

- `commands list` gains the rows of `shelf self environment` and
  `shelf self environment list`, the second marked `built_in` with the
  columns, examples and effects that crumbkit-starmap declares.
- `self environment list` prints
  `tests/fixtures/starmap/self-environment-list.json` whatever its flags,
  so its values are those of that file and not the ones the script was
  started with.
- `books list` fails with `shelf::environment::required_unset`, a failure
  whose message names `SHELF_CATALOGUE_URL`, when that variable is unset,
  and otherwise runs shelf's.

It needs `perl`, whose JSON::PP module edits the `commands list` document.
`tests/model.rs` and the page's journey "environment shelf" run it.

# crumbkit's schemas

These files are copies of `crates/crumbkit/schemas/commands-list-v1.json`,
`schemas-list-v1.json`, `schemas-show-v1.json` and `failure-v1.json` from
crumbkit at commit `cb73bd8ea968033c2119ec91534311234018584e`
(crumbkit/crumbkit#393), which `scripts/install-test-tools.sh` installs shelf and reading
from. They move with that commit.

`commands-list-v1-older.json` is `commands-list-v1.json` from crumbkit at
commit `644f776e29ca7332202653830518e7f41b7d319f` (crumbkit/crumbkit#328),
before crumbkit marked its own commands with `built_in`. The tests of a
tool on an earlier crumbkit read it, and it stays when the commit moves.

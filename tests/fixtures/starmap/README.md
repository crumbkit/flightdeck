# starmap's `self environment list`

The crate crumbkit-starmap, in the starmap repository, provides the
built-in `self environment list`. A crumbkit tool that declares its
environment with starmap mounts it. These files come from crumbkit-starmap
0.4.2 on crates.io, released from starmap commit
`c71a0aa` (crumbkit/starmap#73), on crumbkit 0.6.0. That release adds the
JSON-only fields `type_name`, `unset_rule` and `source_kind` to each row. flightdeck does not depend on crumbkit-starmap, so the
files stand in for a tool that mounts the command.

- `self-environment-list-v1.json` is the document's JSON Schema, as
  `crumbkit::schemas::document_schema(crumbkit_starmap::SELF_ENVIRONMENT_LIST)`
  returns it. crumbkit-starmap checks in no schema file.
- `starmap.toml` declares one variable of each kind that the Environment
  view draws: a default, a set value, a required variable that is unset, a
  withheld secret in a group, a named enumeration and an invalid value.
- `self-environment-list.json` is `shelf self environment list --json
  --full` from a tool named shelf that embeds `starmap.toml`, mounts `self
  usage` beside the command, and ran with `SHELF_DEBUG=true`,
  `SHELF_MIRROR_TOKEN=hunter2`, `SHELF_REGION=moon`,
  `SHELF_PROGRAM=flightdeck`, `XDG_STATE_HOME=/Users/person/.local/state`
  and `TERM=xterm-256color`. The withheld token has no text in it.

They move when flightdeck starts to read a field that a later
crumbkit-starmap prints.

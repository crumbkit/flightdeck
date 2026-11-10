use std::ffi::OsStr;
#[cfg(unix)]
use std::ffi::OsString;
use std::path::Path;

use super::{FreshState, Program, Query};

#[test]
fn a_bare_tool_name_stays_a_name_for_the_path_search() {
    let program = Program::at_launch("shelf", Some(Path::new("/launch")), None).unwrap();

    assert_eq!(program.name(), OsStr::new("shelf"));
}

#[test]
fn a_relative_tool_path_becomes_absolute_against_the_launch_directory() {
    let program = Program::at_launch("tools/shelf", Some(Path::new("/launch")), None).unwrap();

    assert_eq!(
        program.name(),
        Path::new("/launch").join("tools/shelf").as_os_str()
    );
}

#[cfg(unix)]
/// Returns the `PATH` set on a program named `tool` that launched from
/// `launch`, or `None` when the launch set none.
fn search_of(tool: &str, search: &str, launch: Option<&Path>) -> Option<OsString> {
    let program = Program::at_launch(tool, launch, Some(OsStr::new(search))).unwrap();
    let command = program.command::<&str>(&[]);
    command
        .get_envs()
        .find(|(name, _)| *name == "PATH")
        .and_then(|(_, value)| value.map(OsStr::to_os_string))
}

#[cfg(unix)]
#[test]
fn each_path_entry_becomes_absolute_and_an_empty_entry_becomes_the_launch_directory() {
    let search = search_of("shelf", "bin::/usr/bin", Some(Path::new("/launch")));

    assert_eq!(search, Some(OsString::from("/launch/bin:/launch:/usr/bin")));
}

#[cfg(unix)]
#[test]
fn a_bare_name_is_refused_when_a_relative_path_entry_cannot_be_made_absolute() {
    let colon = Some(Path::new("/with:colon"));
    let search = Some(OsStr::new("bin:/usr/bin"));

    assert!(Program::at_launch("shelf", colon, search).is_none());
    assert!(Program::at_launch("shelf", None, search).is_none());
}

#[cfg(unix)]
#[test]
fn a_bare_name_launches_from_a_directory_with_a_separator_when_every_path_entry_is_absolute() {
    let search = search_of("shelf", "/usr/bin", Some(Path::new("/with:colon")));

    assert_eq!(search, Some(OsString::from("/usr/bin")));
}

#[cfg(unix)]
#[test]
fn a_tool_given_by_its_path_keeps_the_inherited_path_when_an_entry_cannot_be_made_absolute() {
    let search = search_of(
        "/tools/shelf",
        "bin:/usr/bin",
        Some(Path::new("/with:colon")),
    );

    assert_eq!(search, None);
}

#[test]
fn each_query_is_a_fixed_argument_vector_and_each_document_query_passes_no_input() {
    let ids = [
        "shelf/books-list/v1".to_owned(),
        "shelf/failure/v1".to_owned(),
    ];

    let vectors: Vec<Vec<&str>> = [
        Query::Version,
        Query::CommandsList,
        Query::SchemasList,
        Query::SchemasShow(&ids),
        Query::FiguresList,
        Query::Usage,
        Query::RunsList,
    ]
    .iter()
    .map(Query::words)
    .collect();

    assert_eq!(
        vectors,
        [
            vec!["--version"],
            vec![
                "commands",
                "list",
                "--json",
                "--full",
                "--all",
                "--no-input"
            ],
            vec!["schemas", "list", "--json", "--full", "--all", "--no-input"],
            vec![
                "schemas",
                "show",
                "--json",
                "--no-input",
                "--",
                "shelf/books-list/v1",
                "shelf/failure/v1",
            ],
            vec![
                "self",
                "figures",
                "list",
                "--json",
                "--full",
                "--all",
                "--no-input"
            ],
            vec!["self", "usage", "--json", "--full", "--no-input"],
            vec![
                "self",
                "runs",
                "list",
                "--json",
                "--full",
                "--all",
                "--no-input"
            ],
        ]
    );
}

/// Returns the value that `program`'s runs get for the variable `name`.
fn variable_of(program: &Program, name: &str) -> Option<std::ffi::OsString> {
    program
        .command::<&str>(&[])
        .get_envs()
        .find(|(each, _)| *each == name)
        .and_then(|(_, value)| value.map(OsStr::to_os_string))
}

#[test]
fn a_fresh_state_replaces_the_programs_state_directory_and_is_gone_once_dropped() {
    let program = Program::new("shelf").env("XDG_STATE_HOME", "/the-persons-state");
    let state = FreshState::new().unwrap();

    let kept_out = state.keep_out(&program);

    let directory = std::path::PathBuf::from(variable_of(&kept_out, "XDG_STATE_HOME").unwrap());
    assert_ne!(directory, Path::new("/the-persons-state"));
    assert!(directory.is_dir(), "{} is missing", directory.display());
    drop(state);
    assert!(
        !directory.exists(),
        "{} is still there",
        directory.display()
    );
}

use std::ffi::{OsStr, OsString};
use std::path::Path;

use super::{Program, Query};

#[test]
fn a_bare_tool_name_stays_a_name_for_the_path_search() {
    let program = Program::at_launch("shelf", Some(Path::new("/launch")), None);

    assert_eq!(program.name(), OsStr::new("shelf"));
}

#[test]
fn a_relative_tool_path_becomes_absolute_against_the_launch_directory() {
    let program = Program::at_launch("tools/shelf", Some(Path::new("/launch")), None);

    assert_eq!(program.name(), OsStr::new("/launch/tools/shelf"));
}

/// Returns the `PATH` that a program launched from `launch` runs with.
fn search_of(search: &str, launch: Option<&Path>) -> Option<OsString> {
    let program = Program::at_launch("shelf", launch, Some(OsStr::new(search)));
    let command = program.command::<&str>(&[]);
    command
        .get_envs()
        .find(|(name, _)| *name == "PATH")
        .and_then(|(_, value)| value.map(OsStr::to_os_string))
}

#[cfg(unix)]
#[test]
fn each_path_entry_becomes_absolute_and_an_empty_entry_becomes_the_launch_directory() {
    let search = search_of("bin::/usr/bin", Some(Path::new("/launch")));

    assert_eq!(search, Some(OsString::from("/launch/bin:/launch:/usr/bin")));
}

#[cfg(unix)]
#[test]
fn without_a_launch_directory_only_absolute_path_entries_remain() {
    assert_eq!(
        search_of("bin::/usr/bin", None),
        Some(OsString::from("/usr/bin"))
    );
}

#[cfg(unix)]
#[test]
fn a_path_entry_that_the_launch_directory_cannot_be_joined_to_is_left_out() {
    let search = search_of("bin:/usr/bin", Some(Path::new("/with:colon")));

    assert_eq!(search, Some(OsString::from("/usr/bin")));
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
            vec!["self", "figures", "list", "--json", "--all", "--no-input"],
        ]
    );
}

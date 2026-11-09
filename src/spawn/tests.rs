use std::ffi::OsStr;
use std::path::Path;

use super::{Program, Query};

#[test]
fn a_bare_tool_name_stays_a_name_for_the_path_search() {
    assert_eq!(Program::at_launch("shelf").name(), OsStr::new("shelf"));
}

#[test]
fn a_relative_tool_path_becomes_absolute_at_launch() {
    let program = Program::at_launch("tools/shelf");

    let path = Path::new(program.name());
    assert!(path.is_absolute(), "{}", path.display());
    assert!(path.ends_with("tools/shelf"), "{}", path.display());
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

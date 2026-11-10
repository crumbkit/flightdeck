use super::Query;

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
        ]
    );
}

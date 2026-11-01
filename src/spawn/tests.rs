use super::{Query, Stdout, read_capped, read_tail};

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

#[test]
fn standard_output_up_to_the_limit_is_kept_whole() {
    let stdout = read_capped(&b"abcd"[..], 4).unwrap();

    assert_eq!(stdout, Stdout::Whole(b"abcd".to_vec()));
}

#[test]
fn standard_output_past_the_limit_is_read_to_its_end_and_dropped() {
    let mut pipe = &b"abcde"[..];

    let stdout = read_capped(&mut pipe, 4).unwrap();

    assert_eq!(stdout, Stdout::OverLimit);
    assert!(pipe.is_empty(), "{} bytes were left unread", pipe.len());
}

#[test]
fn standard_error_keeps_its_last_bytes_up_to_the_limit() {
    let long = vec![b'x'; 3 * super::CHUNK];
    let mut stderr = long.clone();
    stderr.extend_from_slice(b"the end");

    assert_eq!(read_tail(&stderr[..], 7).unwrap(), b"the end");
    assert_eq!(read_tail(&b"short"[..], 7).unwrap(), b"short");
}

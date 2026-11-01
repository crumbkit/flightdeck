use super::{HEAD_LIMIT, HEADER_LIMIT, ends_head, parse_head};
use crate::server::reply::Refusal;
use crate::server::request::Method;

#[test]
fn the_end_of_a_head_is_found_when_its_empty_line_arrives_across_two_reads() {
    let first = b"GET / HTTP/1.1\r\nHost: x\r\n\r";
    let mut buffer = first.to_vec();
    assert!(!ends_head(&buffer, 0));

    buffer.extend_from_slice(b"\n");

    assert!(ends_head(&buffer, first.len()));
}

#[test]
fn the_end_of_a_head_is_found_after_bare_line_feeds() {
    assert!(ends_head(b"GET / HTTP/1.1\nHost: x\n\n", 0));
}

#[test]
fn a_head_parses_into_its_method_path_without_the_query_and_body_length() {
    let bytes =
        b"POST /api/runs?x=1 HTTP/1.1\r\nHost: x\r\nContent-Length: 12\r\n\r\n{\"path\":[]}";

    let (head, end) = parse_head(bytes).unwrap();

    assert_eq!(head.method(), Method::Post);
    assert_eq!(head.path(), "/api/runs");
    assert_eq!(head.body_length(), Some(12));
    assert_eq!(bytes.get(end..), Some(&b"{\"path\":[]}"[..]));
}

#[test]
fn a_head_with_transfer_encoding_or_a_length_that_is_not_one_number_is_malformed() {
    for headers in [
        "Transfer-Encoding: chunked\r\n",
        "Content-Length: 1\r\nContent-Length: 1\r\n",
        "Content-Length: +1\r\n",
        "Content-Length: \r\n",
    ] {
        let bytes = format!("POST / HTTP/1.1\r\nHost: x\r\n{headers}\r\n");

        let refused = parse_head(bytes.as_bytes()).map(|_| ());

        assert_eq!(refused, Err(Refusal::Malformed), "{headers:?}");
    }
}

#[test]
fn a_header_value_that_is_not_utf_8_is_malformed() {
    let refused = parse_head(b"GET / HTTP/1.1\r\nHost: \xff\r\n\r\n").map(|_| ());

    assert_eq!(refused, Err(Refusal::Malformed));
}

#[test]
fn a_length_too_large_for_a_number_reads_as_the_largest_length() {
    let bytes = format!("POST / HTTP/1.1\r\nContent-Length: {}0\r\n\r\n", u64::MAX);

    let (head, _) = parse_head(bytes.as_bytes()).unwrap();

    assert_eq!(head.body_length(), Some(u64::MAX));
}

#[test]
fn a_head_with_too_many_headers_or_over_the_size_limit_is_too_large() {
    let many: String = (0..=HEADER_LIMIT)
        .map(|n| format!("X-{n}: y\r\n"))
        .collect();
    let long = format!("X: {}\r\n", "y".repeat(HEAD_LIMIT));

    for headers in [many, long] {
        let bytes = format!("GET / HTTP/1.1\r\n{headers}\r\n");

        let refused = parse_head(bytes.as_bytes()).map(|_| ());

        assert_eq!(refused, Err(Refusal::HeadTooLarge));
    }
}

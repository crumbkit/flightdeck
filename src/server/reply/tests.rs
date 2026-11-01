use super::{Refusal, start};

#[test]
fn every_refusal_has_its_own_stable_code_and_a_client_error_status() {
    let refusals: Vec<(&str, u16)> = Refusal::ALL
        .iter()
        .map(|refusal| (refusal.code(), refusal.status()))
        .collect();

    assert_eq!(
        refusals,
        [
            ("flightdeck::request::host", 403),
            ("flightdeck::request::cross_site", 403),
            ("flightdeck::request::origin", 403),
            ("flightdeck::request::method", 405),
            ("flightdeck::request::content_type", 415),
            ("flightdeck::request::too_large", 413),
            ("flightdeck::request::head_too_large", 431),
            ("flightdeck::request::malformed", 400),
            ("flightdeck::request::busy", 503),
            ("flightdeck::request::body", 400),
            ("flightdeck::session::unauthorized", 401),
            ("flightdeck::session::code_refused", 401),
            ("flightdeck::request::not_found", 404),
        ]
    );
}

#[test]
fn a_streamed_reply_sends_its_head_and_then_each_line_as_it_is_sent() {
    let mut connection = Vec::new();

    let mut body = start(&mut connection, 200, "application/x-ndjson", None).unwrap();
    body.send(b"{\"line\":1}\n").unwrap();
    body.send(b"{\"line\":2}\n").unwrap();

    let text = String::from_utf8(connection).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    assert!(head.starts_with("HTTP/1.1 200 OK\r\n"), "{head}");
    assert!(
        head.contains("\r\nContent-Type: application/x-ndjson"),
        "{head}"
    );
    assert!(head.ends_with("\r\nConnection: close"), "{head}");
    assert!(!head.contains("Content-Length"), "{head}");
    assert_eq!(body, "{\"line\":1}\n{\"line\":2}\n");
}

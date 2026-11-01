use super::Refusal;

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
            ("flightdeck::request::body", 400),
            ("flightdeck::session::unauthorized", 401),
            ("flightdeck::session::code_refused", 401),
            ("flightdeck::request::not_found", 404),
        ]
    );
}

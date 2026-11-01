use super::{RunRequest, Value};

#[test]
fn the_pages_request_parses_a_flag_one_value_and_a_list() {
    let json = r#"{
        "path": ["books", "show"],
        "values": {"<ID>": ["2", "5"], "--from": "ids.txt", "--all": true}
    }"#;

    let request: RunRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.path(), ["books", "show"]);
    let values: Vec<(&str, &Value)> = request
        .values()
        .iter()
        .map(|(name, value)| (name.as_str(), value))
        .collect();
    assert_eq!(
        values,
        [
            ("--all", &Value::Flag(true)),
            ("--from", &Value::One("ids.txt".to_owned())),
            ("<ID>", &Value::Many(vec!["2".to_owned(), "5".to_owned()])),
        ]
    );
}

#[test]
fn a_request_with_a_field_flightdeck_does_not_read_is_refused() {
    let json = r#"{"path": ["books", "list"], "line": "shelf books list"}"#;

    assert!(serde_json::from_str::<RunRequest>(json).is_err());
}

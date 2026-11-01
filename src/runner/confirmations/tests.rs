use std::time::Duration;

use super::{ConfirmationId, Confirmations, MOST, Problem};
use crate::model::{CommandsList, Tool};
use crate::runner::RunRequest;
use crate::runner::clock::{Clock as _, SystemClock};
use crate::runner::vector::{Vector, build};

/// shelf's `commands list` document at the commit that the install script
/// pins.
const SHELF: &str = include_str!("../../../tests/fixtures/shelf/commands-list.json");

/// Returns the vector of `shelf books remove 2`.
fn remove_2() -> Vector {
    let commands: CommandsList = serde_json::from_str(SHELF).unwrap();
    let tool = Tool::assemble("shelf 0.1.1", commands, Vec::new()).unwrap();
    let request = RunRequest::new(["books", "remove"])
        .value("<ID>", crate::runner::Value::Many(vec!["2".to_owned()]));
    build(&tool, &request).unwrap().1
}

#[test]
fn an_id_prints_as_32_lowercase_hex_digits_and_parses_back() {
    let confirmations = Confirmations::new(Duration::from_secs(600));
    let id = confirmations.issue(remove_2(), SystemClock.now()).unwrap();

    let text = id.to_string();
    let parsed: ConfirmationId = text.parse().unwrap();

    assert_eq!(text.len(), 32);
    assert!(
        text.bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    );
    assert_eq!(parsed, id);
}

#[test]
fn text_that_is_not_32_lowercase_hex_digits_is_no_id() {
    for text in [
        String::new(),
        "abc".to_owned(),
        "A".repeat(32),
        format!("+{}", "0".repeat(31)),
        "0".repeat(33),
    ] {
        assert!(text.parse::<ConfirmationId>().is_err(), "{text}");
    }
}

#[test]
fn the_oldest_id_is_dropped_when_the_store_holds_its_most() {
    let confirmations = Confirmations::new(Duration::from_secs(600));
    let start = SystemClock.now();
    let oldest = confirmations.issue(remove_2(), start).unwrap();
    let mut later = Vec::new();
    for second in 1..=MOST {
        let at = start + Duration::from_secs(second.try_into().unwrap());
        later.push(confirmations.issue(remove_2(), at).unwrap());
    }
    let now = start + Duration::from_secs(300);

    assert_eq!(
        confirmations.take(&oldest, &remove_2(), now),
        Err(Problem::Unknown)
    );
    for id in &later {
        assert_eq!(confirmations.take(id, &remove_2(), now), Ok(()));
    }
}

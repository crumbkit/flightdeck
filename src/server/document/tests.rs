use serde_json::Value;

use super::to_json;
use crate::runner::{ConfirmationProblem, EndFailure, Reason, RunFailure, Takes};
use crate::spawn::SpawnError;

/// crumbkit's schema of its failure document, with the id that a tool
/// named flightdeck prints in place of `crumbkit/failure/v1`.
fn schema() -> jsonschema::Validator {
    let text = include_str!("../../../tests/fixtures/crumbkit/failure-v1.json");
    let mut schema: Value = serde_json::from_str(text).unwrap();
    schema["properties"]["schema"]["const"] = "flightdeck/failure/v1".into();
    jsonschema::validator_for(&schema).unwrap()
}

/// Returns the document of `failure`, after checking it against the schema.
fn document(failure: &dyn crumbkit::Failure) -> Value {
    let document: Value = serde_json::from_slice(&to_json(failure)).unwrap();
    let errors: Vec<String> = schema()
        .iter_errors(&document)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:?} in {document}");
    document
}

#[test]
fn a_refusal_is_a_failure_document_with_its_code_message_fields_and_hint() {
    let refusal = RunFailure::Refused {
        path: vec!["books".to_owned(), "list".to_owned()],
        argument: Some("--all".to_owned()),
        reason: Reason::Arity { takes: Takes::Flag },
    };

    let document = document(&refusal);

    assert_eq!(document["schema"], "flightdeck/failure/v1");
    assert_eq!(document["code"], "flightdeck::run::refused");
    assert_eq!(
        document["message"],
        "--all of books list takes true or false"
    );
    assert_eq!(
        document["fields"],
        serde_json::json!({
            "path": ["books", "list"],
            "argument": "--all",
            "reason": "arity",
            "index": null,
            "takes": "flag",
            "earlier": null,
        })
    );
    assert_eq!(document["recovery"]["kind"], "hint");
}

#[test]
fn a_failure_with_a_source_carries_it_as_its_cause() {
    let failure = EndFailure::Spawn {
        tool: "shelf".to_owned(),
        source: SpawnError::Run(std::io::Error::other("the pipe broke")),
    };

    let document = document(&failure);

    assert_eq!(document["code"], "flightdeck::run::spawn");
    assert!(
        document["cause"]
            .as_str()
            .unwrap()
            .ends_with("the pipe broke"),
        "{document}"
    );
}

#[test]
fn a_failure_with_no_next_step_says_why_and_one_with_no_fields_has_none() {
    let refusal = RunFailure::Confirmation {
        problem: ConfirmationProblem::Expired,
    };

    let closed = document(&RunFailure::Closed);
    let confirmation = document(&refusal);

    assert_eq!(closed["recovery"]["kind"], "none");
    assert!(closed.get("fields").is_none(), "{closed}");
    assert_eq!(confirmation["fields"]["reason"], "expired");
}

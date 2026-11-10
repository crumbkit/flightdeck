//! flightdeck's failure document, `flightdeck/failure/v1`, in the shape of
//! crumbkit's `crumbkit/failure/v1`, for a runner's refusal and for a run
//! that fails at its end.

use clap::CommandFactory as _;
use crumbkit::{Failure, Recovery, RenderCall as _, Spelling};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use crate::cli::Cli;

#[cfg(test)]
mod tests;

/// The id of flightdeck's failure document.
const SCHEMA: &str = "flightdeck/failure/v1";

/// Returns `failure` as the JSON of its failure document.
pub(super) fn to_json(failure: &dyn Failure) -> Vec<u8> {
    // serde_json fails only on a map key that is not a string, and the
    // document's keys are all strings.
    serde_json::to_vec(&Document(failure)).unwrap_or_default()
}

/// A failure that serializes as `flightdeck/failure/v1`: `schema`, `code`,
/// `message`, `cause` when the failure has a source, `fields` when it names
/// at least one value, `recovery`, `alternatives` when it has any, and `url`
/// when it has one, as crumbkit prints a failure with `--json`.
pub(super) struct Document<'a>(pub(super) &'a dyn Failure);

impl Serialize for Document<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let failure = self.0;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("schema", SCHEMA)?;
        map.serialize_entry("code", &Spelling::COLONS.spell(failure.code()))?;
        map.serialize_entry("message", &failure.to_string())?;
        let causes = causes(failure);
        if !causes.is_empty() {
            map.serialize_entry("cause", &causes.join(": "))?;
        }
        let fields = failure.fields();
        if !fields.is_empty() {
            map.serialize_entry("fields", &Fields(&fields))?;
        }
        map.serialize_entry("recovery", &Step(failure.recovery()))?;
        let alternatives: Vec<Step> = failure.alternatives().into_iter().map(Step).collect();
        if !alternatives.is_empty() {
            map.serialize_entry("alternatives", &alternatives)?;
        }
        if let Some(url) = failure.url() {
            map.serialize_entry("url", url)?;
        }
        map.end()
    }
}

/// Returns the message of each error under `failure`, in order.
fn causes(failure: &dyn Failure) -> Vec<String> {
    let mut causes = Vec::new();
    let mut source = failure.source();
    while let Some(error) = source {
        causes.push(error.to_string());
        source = error.source();
    }
    causes
}

/// A failure's fields, which serialize as one JSON object in their order.
struct Fields<'a, 'b>(&'a [crumbkit::Field<'b>]);

impl Serialize for Fields<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for field in self.0 {
            map.serialize_entry(field.name(), field.value())?;
        }
        map.end()
    }
}

/// A recovery, which serializes as an object tagged by `kind`, such as
/// `{"kind":"hint","text":"..."}`.
struct Step(Recovery);

impl Serialize for Step {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        match &self.0 {
            Recovery::Retry => map.serialize_entry("kind", "retry")?,
            Recovery::Hint(text) => {
                map.serialize_entry("kind", "hint")?;
                map.serialize_entry("text", text)?;
            }
            Recovery::None { reason } => {
                map.serialize_entry("kind", "none")?;
                map.serialize_entry("reason", reason)?;
            }
            // RUST-10: a `Run` recovery names a command of flightdeck, so it
            // renders against flightdeck's own command tree, and a call that
            // does not resolve prints as crumbkit prints one. This handles a
            // plain call. It misses a call built with `Call::all`, which needs
            // the document's `all` field, and no failure the server sends has
            // one. A failure-document writer in crumbkit that renders calls
            // replaces this.
            Recovery::Run(call) => match call.render(&mut Cli::command()) {
                Ok(line) => {
                    map.serialize_entry("kind", "run")?;
                    map.serialize_entry("call", &line)?;
                }
                Err(_) => unavailable(&mut map)?,
            },
            _ => unavailable(&mut map)?,
        }
        map.end()
    }
}

/// Writes the recovery of a call that cannot be shown, as crumbkit writes a
/// call that does not resolve: kind `none` with the reason that the suggested
/// command is not available.
fn unavailable<M: SerializeMap>(map: &mut M) -> Result<(), M::Error> {
    map.serialize_entry("kind", "none")?;
    map.serialize_entry("reason", "the suggested command is not available")
}

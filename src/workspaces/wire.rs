//! The documents that mxv prints with `--json --full`, as far as
//! flightdeck reads them. Each type names only the fields that flightdeck
//! copies, so a field that mxv adds, and the paths, logs and sandbox text
//! that it prints, are never read.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The schema id of `mxv list`'s document.
pub(super) const LIST: &str = "mxv/list/v1";

/// The schema id of `mxv status`'s document.
pub(super) const STATUS: &str = "mxv/status/v1";

/// The schema id of `mxv proxy status`'s document.
pub(super) const PROXY_STATUS: &str = "mxv/proxy-status/v1";

/// The schema id of mxv's failure document.
pub(super) const FAILURE: &str = "mxv/failure/v1";

/// The text that `mxv proxy status` prints in place of a URL for a route
/// that another workspace's name shadows.
pub(super) const NOT_SERVED: &str = "(not served)";

/// The schema id that every document of mxv starts with.
#[derive(Debug, Deserialize)]
pub(super) struct Head {
    pub(super) schema: String,
}

/// `mxv list --all --json --full`.
#[derive(Debug, Deserialize)]
pub(super) struct List {
    pub(super) data: Vec<ListRow>,
}

/// One workspace of `mxv list`. A row with a problem names its record in
/// `name` and leaves the other fields empty.
#[derive(Debug, Deserialize)]
pub(super) struct ListRow {
    pub(super) name: String,
    #[serde(default)]
    pub(super) branch: Option<String>,
    #[serde(default)]
    pub(super) checkouts: Vec<ListCheckout>,
    #[serde(default)]
    pub(super) ports: BTreeMap<String, u16>,
    #[serde(default)]
    pub(super) problem: Option<String>,
}

/// One checkout of a workspace in `mxv list`.
#[derive(Debug, Deserialize)]
pub(super) struct ListCheckout {
    pub(super) name: String,
}

/// `mxv status --workspace=<name> --json --full`.
#[derive(Debug, Deserialize)]
pub(super) struct Status {
    pub(super) data: Vec<Unit>,
    pub(super) summary: StatusSummary,
}

/// One unit of `mxv status`.
#[derive(Debug, Deserialize)]
pub(super) struct Unit {
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) state: String,
    #[serde(default)]
    pub(super) ready: Option<bool>,
    #[serde(default)]
    pub(super) code: Option<i64>,
    #[serde(default)]
    pub(super) signal: Option<i64>,
    #[serde(default)]
    pub(super) reason: Option<String>,
    #[serde(default)]
    pub(super) ports: Vec<String>,
}

/// What `mxv status` says above its units.
#[derive(Debug, Deserialize)]
pub(super) struct StatusSummary {
    #[serde(default)]
    pub(super) manifest_error: Option<String>,
    #[serde(default)]
    pub(super) manifest_error_code: Option<String>,
    #[serde(default)]
    pub(super) checkouts: Vec<StatusCheckout>,
    #[serde(default)]
    pub(super) pinned: bool,
}

/// One checkout of a workspace in `mxv status`.
#[derive(Debug, Deserialize)]
pub(super) struct StatusCheckout {
    pub(super) name: String,
    #[serde(default)]
    pub(super) branch: Option<String>,
    #[serde(default)]
    pub(super) commit: Option<String>,
    #[serde(default)]
    pub(super) changes: Option<u64>,
    #[serde(default)]
    pub(super) problem: Option<String>,
}

/// `mxv proxy status --json --full`.
#[derive(Debug, Deserialize)]
pub(super) struct ProxyStatus {
    pub(super) data: Vec<Route>,
}

/// One route of `mxv proxy status`.
#[derive(Debug, Deserialize)]
pub(super) struct Route {
    pub(super) route: String,
    pub(super) workspace: String,
    pub(super) url: String,
    #[serde(default)]
    pub(super) unit: Option<String>,
}

/// mxv's failure document.
#[derive(Debug, Deserialize)]
pub(super) struct Failure {
    pub(super) code: String,
    pub(super) message: String,
}

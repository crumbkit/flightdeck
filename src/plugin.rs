//! The flightdeck Board plugin: its manifest and the schema of
//! `flightdeck/workflow-graph/v1`, embedded in the binary, the install that
//! writes them where Board finds plugins, and the one capability,
//! `flightdeck.check`, that Board calls.
//!
//! [`install`] writes `<data>/board/plugins/flightdeck/` in two renames. It
//! writes a temporary directory beside it and reads back what it wrote. It
//! then renames the old plugin aside to `.flightdeck-old-<pid>` and renames
//! the new directory into place. When the second rename fails, it renames
//! the old plugin back. Between the two renames no plugin is at
//! `flightdeck/`, so a Board that looks then finds none, and a crash there
//! leaves the old plugin whole under `.flightdeck-old-<pid>`, where a
//! person can rename it back.
//! Enabling the plugin in a board stays a separate `board plugin enable
//! flightdeck`, so installing changes no board.
//!
//! [`call`] answers one `board/call/v1` request with one
//! `board/call-result/v1` or `board/error/v1`, as docs/plugins.md in Board
//! describes. The check reads the board through `board query`, run as a
//! child in the board's root, and never opens Board's index.

use std::fmt;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use crumbkit::{Code, Document, Failure, Prose};
use crumbkit_process::Cancel;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::spawn::{BoardAt, GRAPHS_UNDER, Program};
use crate::workflows::{self, Board, Checked, Slug, WorkflowsError};

#[cfg(unix)]
#[cfg(test)]
mod tests;

/// The plugin's name, which is the name of its directory.
pub const NAME: &str = "flightdeck";

/// The plugin's manifest, a `board/plugin-manifest/v1`.
pub const MANIFEST: &str = include_str!("../plugin/plugin.json");

/// The schema of `flightdeck/workflow-graph/v1`, which [`crate::workflow_graph::schema`]
/// generates and a test keeps equal.
pub const SCHEMA: &str = include_str!("../plugin/types/flightdeck/workflow-graph/v1.schema.json");

/// Where the schema goes inside the plugin's directory.
const SCHEMA_PATH: &str = "types/flightdeck/workflow-graph/v1.schema.json";

/// The most bytes of a request that a call reads.
const REQUEST_LIMIT: u64 = 1024 * 1024;

/// The code of a machine with no data directory to install into.
pub const NO_DATA_HOME: Code = Code::from_spelling("flightdeck::plugin::no_data_home");

/// The code of an install that could not write the plugin's directory.
pub const INSTALL_FAILED: Code = Code::from_spelling("flightdeck::plugin::install_failed");

/// The document that `flightdeck plugin install` prints.
pub(crate) const PLUGIN_INSTALL: Document<Prose<Installed>> = Document::new(
    "flightdeck",
    "plugin-install",
    1,
    "Where the flightdeck Board plugin was written, and whether it replaced an earlier one.",
);

/// What an install wrote.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Installed {
    /// The plugin's directory.
    pub path: String,
    /// The plugin's version, from its manifest.
    pub version: String,
    /// Whether an earlier plugin directory was replaced.
    pub replaced: bool,
}

impl fmt::Display for Installed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let verb = if self.replaced {
            "replaced"
        } else {
            "installed"
        };
        writeln!(
            f,
            "{verb} the flightdeck plugin {} at {}",
            self.version, self.path
        )?;
        write!(
            f,
            "run `board plugin enable flightdeck` in each board that holds workflow graphs"
        )
    }
}

/// Why an install wrote nothing.
#[derive(Debug, Failure)]
pub enum InstallError {
    /// Neither `XDG_DATA_HOME` nor `HOME` names an absolute directory.
    #[failure(
        code = NO_DATA_HOME,
        hint = "set XDG_DATA_HOME or HOME to an absolute directory",
    )]
    NoDataHome,
    /// A step of the install failed; the old plugin directory is whole.
    #[failure(
        code = INSTALL_FAILED,
        hint = "make the directory writable or free some space, then install again",
        field(path = path.as_str(), step = *step),
    )]
    Failed {
        /// The path that the failed step wrote or renamed.
        path: String,
        /// The step that failed, such as `write`.
        step: &'static str,
        /// The operating system's reason.
        reason: String,
    },
}

impl fmt::Display for InstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDataHome => f.write_str(
                "neither XDG_DATA_HOME nor HOME names an absolute directory to install into",
            ),
            Self::Failed { path, step, reason } => {
                write!(
                    f,
                    "cannot {step} {path}: {reason}; the old plugin is unchanged"
                )
            }
        }
    }
}

impl std::error::Error for InstallError {}

/// Returns the directory that holds Board's plugins: `$XDG_DATA_HOME/board/plugins`,
/// or `$HOME/.local/share/board/plugins` when `XDG_DATA_HOME` is unset or
/// relative, as Board finds them.
///
/// # Errors
///
/// Returns [`InstallError::NoDataHome`] when neither names an absolute
/// directory.
pub fn plugins_directory(
    data_home: Option<&Path>,
    home: Option<&Path>,
) -> Result<PathBuf, InstallError> {
    let data = match (data_home, home) {
        (Some(data), _) if data.is_absolute() => data.to_path_buf(),
        (_, Some(home)) if home.is_absolute() => home.join(".local/share"),
        _ => return Err(InstallError::NoDataHome),
    };
    Ok(data.join("board/plugins"))
}

/// Writes the plugin into `plugins`, the directory of Board's plugins, and
/// returns where.
///
/// # Errors
///
/// Returns [`InstallError::Failed`] when a directory cannot be made, a file
/// cannot be written or read back, or a rename fails. The temporary
/// directory is removed and the old plugin directory is renamed back, or,
/// when that rename fails too, left whole under the name the error gives.
pub fn install(plugins: &Path) -> Result<Installed, InstallError> {
    let failed = |path: &Path, step: &'static str| {
        let path = path.display().to_string();
        move |error: io::Error| InstallError::Failed {
            path,
            step,
            reason: error.to_string(),
        }
    };
    std::fs::create_dir_all(plugins).map_err(failed(plugins, "make"))?;
    let staged = tempfile::Builder::new()
        .prefix(".flightdeck-new-")
        .tempdir_in(plugins)
        .map_err(failed(plugins, "write into"))?;
    write_plugin(staged.path())?;

    let target = plugins.join(NAME);
    let replaced = target.symlink_metadata().is_ok();
    let aside = plugins.join(format!(".flightdeck-old-{}", std::process::id()));
    if replaced {
        std::fs::rename(&target, &aside).map_err(failed(&target, "move aside"))?;
    }
    if let Err(error) = std::fs::rename(staged.path(), &target) {
        if replaced {
            // The old directory goes back where it was. When even that
            // fails, it stays whole beside, under the name the error names.
            std::fs::rename(&aside, &target).map_err(failed(&aside, "move back"))?;
        }
        return Err(failed(&target, "rename into place")(error));
    }
    // The staged directory is now the plugin, so nothing is left to remove.
    drop(staged.keep());
    if replaced {
        let old = aside.symlink_metadata();
        let removed = if old.is_ok_and(|old| old.is_dir()) {
            std::fs::remove_dir_all(&aside)
        } else {
            std::fs::remove_file(&aside)
        };
        // RUST-10: an old directory that cannot be removed stays beside
        // the new one under a hidden name. It handles a plugin that was
        // replaced, and misses only the cleanup, which the next install
        // retries for its own name. A sweep of old names replaces it.
        drop(removed);
    }
    Ok(Installed {
        path: target.display().to_string(),
        version: version(),
        replaced,
    })
}

/// Writes the manifest and the schema into `directory`, then reads both
/// back and parses them, so a short write fails here and not in Board.
fn write_plugin(directory: &Path) -> Result<(), InstallError> {
    let files = [("plugin.json", MANIFEST), (SCHEMA_PATH, SCHEMA)];
    for (relative, text) in files {
        let path = directory.join(relative);
        let failed = |step: &'static str| {
            let shown = path.display().to_string();
            move |error: io::Error| InstallError::Failed {
                path: shown,
                step,
                reason: error.to_string(),
            }
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(failed("make"))?;
        }
        std::fs::write(&path, text).map_err(failed("write"))?;
        let back = std::fs::read(&path).map_err(failed("read back"))?;
        let parsed = serde_json::from_slice::<Value>(&back).is_ok();
        if back != text.as_bytes() || !parsed {
            return Err(InstallError::Failed {
                path: path.display().to_string(),
                step: "read back",
                reason: "the file differs from what was written".to_owned(),
            });
        }
    }
    Ok(())
}

/// Returns the version that the manifest names.
fn version() -> String {
    serde_json::from_str::<Value>(MANIFEST)
        .ok()
        .and_then(|manifest| manifest.get("version")?.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The parts of a `board/call/v1` request that a call reads.
#[derive(Deserialize)]
struct Request {
    #[serde(rename = "$type")]
    tag: String,
    capability: String,
    #[serde(default)]
    input: Map<String, Value>,
    board: Root,
}

/// The board that a call reads.
#[derive(Deserialize)]
struct Root {
    root: PathBuf,
}

/// The answer of one call: the document to print and whether it is an
/// error, which exits 1.
#[derive(Debug)]
pub struct Answer {
    /// The `board/call-result/v1` or `board/error/v1`, as one line.
    pub document: Vec<u8>,
    /// Whether the document is an error.
    pub failed: bool,
}

/// Returns the error answer with `code`, `message` and `fields`.
fn refused(code: &str, message: String, fields: Value, hint: Option<&str>) -> Answer {
    let mut document = json!({
        "$type": "board/error/v1",
        "code": code,
        "message": message,
        "fields": fields,
    });
    if let (Some(hint), Some(object)) = (hint, document.as_object_mut()) {
        object.insert("hint".to_owned(), Value::from(hint));
    }
    Answer {
        document: line(&document),
        failed: true,
    }
}

/// Returns `document` as one line of JSON.
fn line(document: &Value) -> Vec<u8> {
    // serde_json fails only on a map key that is not a string.
    let mut bytes = serde_json::to_vec(document).unwrap_or_default();
    bytes.push(b'\n');
    bytes
}

/// Answers the request that `input` reads, at most [`REQUEST_LIMIT`]
/// bytes, running `program`, the `BOARD_BIN` that Board gives, or `board`.
pub fn call(input: impl io::Read, program: Option<&std::ffi::OsStr>) -> Answer {
    let mut bytes = Vec::new();
    if let Err(error) = input.take(REQUEST_LIMIT).read_to_end(&mut bytes) {
        return bad_input(format!("cannot read the request: {error}"));
    }
    let request: Request = match serde_json::from_slice(&bytes) {
        Ok(request) => request,
        Err(error) => return bad_input(format!("the request is not a board/call/v1: {error}")),
    };
    if request.tag != "board/call/v1" {
        return bad_input(format!(
            "the request is a {}, not a board/call/v1",
            request.tag
        ));
    }
    if request.capability != "check" {
        return refused(
            "flightdeck::call::no_capability",
            format!(
                "the flightdeck plugin offers check, not {}",
                request.capability
            ),
            json!({"capability": request.capability}),
            Some("call flightdeck.check"),
        );
    }
    let Some(path) = request.input.get("path").and_then(Value::as_str) else {
        return bad_input(
            "the check takes a path, such as /workflow-graphs/development-flow".to_owned(),
        );
    };
    let Some(slug) = path
        .strip_prefix(GRAPHS_UNDER)
        .and_then(|slug| Slug::new(slug).ok())
    else {
        return refused(
            "flightdeck::check::bad_input",
            format!("{path} is not a graph's path, /workflow-graphs/<slug>"),
            json!({"path": path}),
            Some("give the path of a graph, such as /workflow-graphs/development-flow"),
        );
    };
    let board = Board::new(
        Program::new(program.unwrap_or_else(|| std::ffi::OsStr::new("board"))),
        BoardAt::Directory(request.board.root),
    );
    match workflows::check(&board, &slug, &Cancel::new()) {
        Ok(Checked::Problems(problems)) => Answer {
            document: line(&json!({
                "$type": "board/call-result/v1",
                "output": {"path": path, "problems": problems},
            })),
            failed: false,
        },
        Ok(Checked::Degraded(code)) => refused(
            "flightdeck::check::degraded",
            format!(
                "Board marks {path} degraded ({}), so its schema refuses it before the check",
                code.as_deref().unwrap_or("no code")
            ),
            json!({"path": path, "board_code": code}),
            Some("run `board check` and fix what it names, then check again"),
        ),
        Err(WorkflowsError::NotFound { .. }) => refused(
            "flightdeck::check::not_found",
            format!("the board holds no graph at {path}"),
            json!({"path": path}),
            Some("run `flightdeck workflows list` to see the graphs"),
        ),
        Err(error) => refused(
            "flightdeck::board::failed",
            error.to_string(),
            json!({"path": path}),
            None,
        ),
    }
}

/// Returns the refusal of a request that is not one the check reads.
fn bad_input(message: String) -> Answer {
    refused("flightdeck::check::bad_input", message, json!({}), None)
}

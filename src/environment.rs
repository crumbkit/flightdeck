//! The environment variables that flightdeck reads itself, which
//! `starmap.toml` declares. `main` reads the process environment once, and
//! [`Environment`] reads the declared variables from that snapshot (BASE-6).
//! crumbkit reads its own variables, such as `FLIGHTDECK_OUTPUT_BUDGET`, and
//! `flightdeck self environment list` lists them after these.

use std::ffi::{OsStr, OsString};

#[cfg(test)]
mod tests;

/// The variables that flightdeck reads, as `starmap.toml` declares them.
#[derive(Debug, starmap::Env)]
#[starmap(spec = "starmap.toml")]
pub(crate) struct Environment {
    /// The search path that finds a tool given by a bare name, or `None`
    /// when `PATH` is absent. An empty `PATH` is a value.
    #[starmap("PATH")]
    path: Option<OsString>,
    /// The port that the server listens on, or `None` when the system
    /// picks one.
    #[starmap("FLIGHTDECK_PORT")]
    port: Option<starmap::Port>,
    /// The mxv program that the workspaces queries run, or `None` when
    /// flightdeck runs `mxv` from `PATH`.
    #[starmap("FLIGHTDECK_MXV")]
    mxv: Option<OsString>,
}

impl Environment {
    /// Returns the search path, or `None` when `PATH` is absent.
    pub(crate) fn path(&self) -> Option<&OsStr> {
        self.path.as_deref()
    }

    /// Returns the port that `FLIGHTDECK_PORT` names, or `None` when it is
    /// absent.
    pub(crate) fn port(&self) -> Option<u16> {
        self.port.map(starmap::Port::get)
    }

    /// Returns the mxv program that `FLIGHTDECK_MXV` names, or `None` when
    /// it is absent.
    pub(crate) fn mxv(&self) -> Option<&OsStr> {
        self.mxv.as_deref()
    }
}

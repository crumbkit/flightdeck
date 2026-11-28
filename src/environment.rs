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
    /// The Board program that the workflows queries run, or `None` when
    /// flightdeck runs `board` from `PATH`.
    #[starmap("FLIGHTDECK_BOARD_PROGRAM")]
    board_program: Option<OsString>,
    /// The directory of the board that the workflows queries read, or
    /// `None` for the person's own board.
    #[starmap("FLIGHTDECK_BOARD")]
    board: Option<OsString>,
    /// The Board that Board gives a plugin it calls, or `None`.
    #[starmap("BOARD_BIN")]
    board_bin: Option<OsString>,
    /// The directory of the person's data files, or `None`.
    #[starmap("XDG_DATA_HOME")]
    data_home: Option<starmap::Secret>,
    /// The person's home directory, or `None`.
    #[starmap("HOME")]
    home: Option<starmap::Secret>,
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

    /// Returns the Board program that `FLIGHTDECK_BOARD_PROGRAM` names, or
    /// `None` when it is absent.
    pub(crate) fn board_program(&self) -> Option<&OsStr> {
        self.board_program.as_deref()
    }

    /// Returns the board directory that `FLIGHTDECK_BOARD` names, or `None`
    /// when it is absent.
    pub(crate) fn board(&self) -> Option<&OsStr> {
        self.board.as_deref()
    }

    /// Returns the Board that `BOARD_BIN` names, or `None` when it is
    /// absent.
    pub(crate) fn board_bin(&self) -> Option<&OsStr> {
        self.board_bin.as_deref()
    }

    /// Returns the directory that `XDG_DATA_HOME` names, or `None`.
    pub(crate) fn data_home(&self) -> Option<&str> {
        self.data_home.as_ref().map(starmap::Secret::expose)
    }

    /// Returns the directory that `HOME` names, or `None`.
    pub(crate) fn home(&self) -> Option<&str> {
        self.home.as_ref().map(starmap::Secret::expose)
    }
}

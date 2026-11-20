//! The name of an mxv workspace, checked before flightdeck passes it to
//! `mxv status`.

use std::fmt;
use std::str::FromStr;

#[cfg(test)]
mod tests;

/// The name of an mxv workspace. It is not empty, is not `.` or `..`, does
/// not start with `-`, and holds no control character and no `/`, so it
/// reaches mxv as one value of `--workspace` and names one directory.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkspaceName(String);

/// Why a workspace name was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    /// The name is empty.
    Empty,
    /// The name starts with `-`, so a program could read it as an option.
    Dash,
    /// The name holds a control character.
    Control,
    /// The name holds a `/`, and a workspace is one directory.
    Slash,
    /// The name is `.` or `..`, which name a directory and its parent.
    Dots,
    /// The name in a page's address has a broken `%` escape or is not UTF-8.
    Escape,
}

impl WorkspaceName {
    /// Returns `text` as a workspace name.
    ///
    /// # Errors
    ///
    /// Returns the [`NameError`] that says why `text` is not a name.
    pub fn new(text: &str) -> Result<Self, NameError> {
        if text.is_empty() {
            return Err(NameError::Empty);
        }
        if text.starts_with('-') {
            return Err(NameError::Dash);
        }
        if text.chars().any(char::is_control) {
            return Err(NameError::Control);
        }
        if text.contains('/') {
            return Err(NameError::Slash);
        }
        if text == "." || text == ".." {
            return Err(NameError::Dots);
        }
        Ok(Self(text.to_owned()))
    }

    /// Returns the name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for WorkspaceName {
    type Err = NameError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

impl fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl NameError {
    /// Returns the reason's one word, which a failure's fields name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Dash => "dash",
            Self::Control => "control",
            Self::Slash => "slash",
            Self::Dots => "dots",
            Self::Escape => "escape",
        }
    }
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "a workspace name cannot be empty",
            Self::Dash => "a workspace name cannot start with -",
            Self::Control => "a workspace name cannot hold a control character",
            Self::Slash => "a workspace name cannot hold a /",
            Self::Dots => "a workspace name cannot be . or ..",
            Self::Escape => "a workspace name in an address must be percent-encoded UTF-8",
        })
    }
}

impl std::error::Error for NameError {}

//! The opener page: a file that sends the browser to the page with the
//! launch code in the URL's fragment.
//!
//! The browser opens a file, so the launch code never appears in a
//! process's arguments. A browser never sends a fragment to a server, so
//! the code never appears in a request line or a log either. The file lives
//! in a new directory that only the user can enter, and the page can read
//! only the user, until the [`Opener`] is dropped and removes both.

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write as _};
use std::path::{Path, PathBuf};

use secrecy::{ExposeSecret as _, SecretString};
use tempfile::TempDir;

/// The opener page's file name in its directory.
const PAGE: &str = "open.html";

/// The opener page and its directory, which dropping it removes.
#[derive(Debug)]
pub struct Opener {
    /// The directory, which flightdeck only holds: dropping it removes the
    /// directory and the page.
    _directory: TempDir,
    page: PathBuf,
}

impl Opener {
    /// Writes the opener page that sends the browser to `origin` with
    /// `code` in the fragment, in a new directory under the system's
    /// temporary directory. On Unix the directory has mode 0700 and the page
    /// mode 0600.
    ///
    /// # Errors
    ///
    /// Returns the error of creating the directory or writing the page.
    pub fn write(origin: &str, code: &SecretString) -> io::Result<Self> {
        let directory = private_directory()?;
        let page = directory.path().join(PAGE);
        let mut file = BufWriter::new(private_file(&page)?);
        // `location.replace` keeps the file out of the tab's history. The
        // link is for a browser that runs no script.
        write!(
            file,
            "<!doctype html>\n\
             <html lang=\"en\">\n\
             <meta charset=\"utf-8\">\n\
             <meta name=\"referrer\" content=\"no-referrer\">\n\
             <title>Opening flightdeck</title>\n\
             <script>location.replace(\"{origin}/#code={code}\");</script>\n\
             <p><a href=\"{origin}/#code={code}\">Open flightdeck</a></p>\n\
             </html>\n",
            code = code.expose_secret(),
        )?;
        file.into_inner().map_err(io::IntoInnerError::into_error)?;
        Ok(Self {
            _directory: directory,
            page,
        })
    }

    /// Returns the path of the opener page.
    #[must_use]
    pub fn page(&self) -> &Path {
        &self.page
    }
}

/// Creates a new directory under the system's temporary directory that only
/// the user can enter.
fn private_directory() -> io::Result<TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("flightdeck-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder.tempdir()
}

/// Creates the new file `path`, which only the user can read or write.
fn private_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options.open(path)
}

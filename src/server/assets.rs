//! The files of the page, embedded in the binary, so the server reads no
//! file at run time.

/// One embedded file of the page.
#[derive(Debug)]
pub(super) struct Asset {
    path: &'static str,
    content_type: &'static str,
    body: &'static [u8],
}

impl Asset {
    /// Returns the asset's content type, such as `text/html; charset=utf-8`.
    pub(super) const fn content_type(&self) -> &'static str {
        self.content_type
    }

    /// Returns the asset's bytes.
    pub(super) const fn body(&self) -> &'static [u8] {
        self.body
    }
}

/// Every asset of the page.
///
/// RUST-10: the set holds one placeholder page that says the page is not
/// built yet. It serves the routes and the headers that the real page
/// needs, and it misses the page itself, so the launch code stays in the
/// address bar unexchanged. The React page that a later pull request (H4)
/// builds and embeds replaces it.
pub(super) const ALL: &[Asset] = &[Asset {
    path: "/",
    content_type: "text/html; charset=utf-8",
    body: include_bytes!("assets/index.html"),
}];

/// Returns the asset at `path`, such as `/`.
pub(super) fn find(path: &str) -> Option<&'static Asset> {
    ALL.iter().find(|asset| asset.path == path)
}

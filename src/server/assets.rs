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

/// Every asset of the page: the files that `pnpm run build` writes to
/// `ui/dist`, under fixed names. A test requests every file in that
/// directory, so a built file missing here fails it.
pub(super) const ALL: &[Asset] = &[
    Asset {
        path: "/",
        content_type: "text/html; charset=utf-8",
        body: include_bytes!("../../ui/dist/index.html"),
    },
    Asset {
        path: "/assets/app.js",
        content_type: "text/javascript; charset=utf-8",
        body: include_bytes!("../../ui/dist/assets/app.js"),
    },
    Asset {
        path: "/assets/index.css",
        content_type: "text/css; charset=utf-8",
        body: include_bytes!("../../ui/dist/assets/index.css"),
    },
];

/// Returns the asset at `path`, such as `/`.
pub(super) fn find(path: &str) -> Option<&'static Asset> {
    ALL.iter().find(|asset| asset.path == path)
}

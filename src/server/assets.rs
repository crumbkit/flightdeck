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
    // The libraries that the page uses, such as React, which the first load
    // reads beside the page's own file.
    Asset {
        path: "/assets/shared.js",
        content_type: "text/javascript; charset=utf-8",
        body: include_bytes!("../../ui/dist/assets/shared.js"),
    },
    // The bundler's runtime, which the first load reads too.
    Asset {
        path: "/assets/rolldown-runtime.js",
        content_type: "text/javascript; charset=utf-8",
        body: include_bytes!("../../ui/dist/assets/rolldown-runtime.js"),
    },
    // The usage view, which the page loads the first time it opens.
    Asset {
        path: "/assets/Usage.js",
        content_type: "text/javascript; charset=utf-8",
        body: include_bytes!("../../ui/dist/assets/Usage.js"),
    },
    // The usage view's weight tab, which the page loads the first time it
    // opens.
    Asset {
        path: "/assets/Weight.js",
        content_type: "text/javascript; charset=utf-8",
        body: include_bytes!("../../ui/dist/assets/Weight.js"),
    },
    // The usage view's both tab, which the page loads the first time it
    // opens.
    Asset {
        path: "/assets/Both.js",
        content_type: "text/javascript; charset=utf-8",
        body: include_bytes!("../../ui/dist/assets/Both.js"),
    },
    // The help-page measurement that the weight and both tabs share.
    Asset {
        path: "/assets/measure.js",
        content_type: "text/javascript; charset=utf-8",
        body: include_bytes!("../../ui/dist/assets/measure.js"),
    },
    Asset {
        path: "/assets/index.css",
        content_type: "text/css; charset=utf-8",
        body: include_bytes!("../../ui/dist/assets/index.css"),
    },
    Asset {
        path: "/fonts/jetbrains-mono-latin-wght-normal.woff2",
        content_type: "font/woff2",
        body: include_bytes!("../../ui/dist/fonts/jetbrains-mono-latin-wght-normal.woff2"),
    },
    Asset {
        path: "/fonts/ibm-plex-sans-latin-400-normal.woff2",
        content_type: "font/woff2",
        body: include_bytes!("../../ui/dist/fonts/ibm-plex-sans-latin-400-normal.woff2"),
    },
    Asset {
        path: "/fonts/ibm-plex-sans-latin-500-normal.woff2",
        content_type: "font/woff2",
        body: include_bytes!("../../ui/dist/fonts/ibm-plex-sans-latin-500-normal.woff2"),
    },
    Asset {
        path: "/fonts/ibm-plex-sans-latin-600-normal.woff2",
        content_type: "font/woff2",
        body: include_bytes!("../../ui/dist/fonts/ibm-plex-sans-latin-600-normal.woff2"),
    },
];

/// Returns the asset at `path`, such as `/`.
pub(super) fn find(path: &str) -> Option<&'static Asset> {
    ALL.iter().find(|asset| asset.path == path)
}

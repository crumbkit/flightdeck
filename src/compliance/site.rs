//! The Compliance site: one self-contained HTML file that draws the
//! Compliance, repository and Rules pages from the document inlined in it.
//!
//! `pnpm run build` in `ui/` writes `ui/dist/compliance.html` with its
//! script and styles inlined and a marker where the document goes. The
//! binary embeds that file, and [`page`] replaces the marker with the
//! document as a `<script type="application/json">` block, so the page
//! opens with no server, no Board and no other file.

/// The name of the page that `--out` writes beside `compliance.json`.
pub const PAGE_FILE: &str = "compliance.html";

/// The most bytes a Flare share holds, 4 MiB. A larger page is still
/// written, with the warning [`OVER_FLARE_CAP`].
pub const FLARE_CAP: u64 = 4 * 1024 * 1024;

/// The code of the warning that a written page is over [`FLARE_CAP`].
pub const OVER_FLARE_CAP: &str = "flightdeck::compliance::over_flare_cap";

/// The comment in the built page that the document replaces.
pub const MARKER: &str = "<!--flightdeck:compliance-document-->";

/// The id of the script element that holds the document.
pub const DOCUMENT_ID: &str = "compliance-document";

/// The page that `pnpm run build` writes, embedded in the binary.
pub const BUNDLE: &[u8] = include_bytes!("../../ui/dist/compliance.html");

/// Returns `template` with its [`MARKER`] replaced by `json`, the
/// document, inside a `<script type="application/json">` element. Every
/// `<` in the document is written as `\u003c`, which JSON reads as the same
/// character, so no text in it, such as a note that holds `</script>`, can
/// end the element. Returns `None` when `template` holds no marker.
#[must_use]
pub fn page(template: &[u8], json: &[u8]) -> Option<Vec<u8>> {
    let marker = MARKER.as_bytes();
    let at = template
        .windows(marker.len())
        .position(|window| window == marker)?;
    let (before, rest) = template.split_at_checked(at)?;
    let after = rest.strip_prefix(marker)?;
    let mut out = Vec::with_capacity(template.len() + json.len() + 64);
    out.extend_from_slice(before);
    out.extend_from_slice(b"<script type=\"application/json\" id=\"");
    out.extend_from_slice(DOCUMENT_ID.as_bytes());
    out.extend_from_slice(b"\">");
    for &byte in json.trim_ascii_end() {
        if byte == b'<' {
            out.extend_from_slice(b"\\u003c");
        } else {
            out.push(byte);
        }
    }
    out.extend_from_slice(b"</script>");
    out.extend_from_slice(after);
    Some(out)
}

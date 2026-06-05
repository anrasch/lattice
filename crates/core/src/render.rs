//! Markdown rendering: comrak to HTML, then ammonia sanitize. One renderer for
//! the whole app so the webview never runs a second markdown parser.

use comrak::{markdown_to_html, ComrakOptions};

/// Render markdown to sanitized HTML. GFM extensions on; raw HTML is rendered
/// by comrak then stripped to a safe subset by ammonia (defense in depth).
pub fn render_html(markdown: &str) -> String {
    let mut opts = ComrakOptions::default();
    opts.extension.strikethrough = true;
    opts.extension.table = true;
    opts.extension.tasklist = true;
    opts.extension.autolink = true;
    opts.render.unsafe_ = true; // allow raw HTML through comrak; ammonia sanitizes next
    let raw = markdown_to_html(markdown, &opts);
    ammonia::clean(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_headings_and_links_and_strips_scripts() {
        let html =
            render_html("# Title\n\nText with [a link](other.md).\n\n<script>alert(1)</script>");
        assert!(html.contains("<h1>"));
        assert!(html.contains("Title"));
        assert!(html.contains("href=\"other.md\""));
        assert!(!html.contains("<script>")); // sanitized
    }

    #[test]
    fn renders_fenced_code() {
        let html = render_html("```rust\nfn main() {}\n```\n");
        assert!(html.contains("<code"));
        assert!(html.contains("fn main"));
    }
}

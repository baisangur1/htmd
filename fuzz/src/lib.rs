//! Code shared by the fuzz targets; see `fuzzer/fuzzer-spec.md`.

pub mod compare;
pub mod generator;
pub mod options;
pub mod oracle;
pub mod scope;
pub mod text;

use pulldown_cmark::{Options, Parser, html};

/// pulldown-cmark with `ENABLE_TABLES | ENABLE_MATH` only.
pub fn render(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, Options::ENABLE_TABLES | Options::ENABLE_MATH);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

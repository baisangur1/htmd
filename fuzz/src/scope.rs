//! "Scope".

use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::TreeBuilderOpts;
use html5ever::{ParseOpts, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use crate::compare::{TYPE_6, is_inline};

/// The rows of the "Scope" table, in its order: what the DOM holds, then the
/// reason.
const ROWS: [&str; 9] = [
    "<script> or <style> inline: Type 1 content: mis-translated inline",
    "<pre> or <textarea> inline with whitespace other than single spaces: Type 1 content: \"newlines … are lost\"",
    "blank line in a comment, <script> or <style> in a type 6 HTML block: Special case: unspecified",
    "type 1 end condition then a line ending in <pre>: Special case: unspecified",
    "raw text of <iframe>, <noembed>, <noframes>, <xmp>, <plaintext> or <noscript>: Special case: unspecified",
    "<template>: Not specified: its content is not among its children",
    "CR in <pre>, <textarea>, or a title or alt: Not specified: the HTML parser reads it back as a line feed",
    "doctype, child of <head>, <frameset> or comment after <body>: Not specified: the document parse differs from the fragment parse",
    "math holding %: \"Math\": comments are replaced, changing source text",
];

/// The rows after which `translation_approach.md` changes the text, so that
/// "Text" exempts the input.
const TEXT_ROWS: [usize; 5] = [0, 2, 3, 4, 8];

/// Some(reason) with the row of the "Scope" table which rejects h0.
pub fn out_of_scope(h0: &str, scripting_enabled: bool) -> Option<&'static str> {
    let row = Walk::run(h0, scripting_enabled)
        .rows
        .iter()
        .position(|&matched| matched)?;
    Some(ROWS[row])
}

/// Whether "Text" exempts `h0`.
pub(crate) fn text_exempt(h0: &str, scripting_enabled: bool) -> bool {
    let walk = Walk::run(h0, scripting_enabled);
    walk.parsers_disagree || TEXT_ROWS.iter().any(|&row| walk.rows[row])
}

/// The context of "Translating HTML nodes" in `translation_approach.md`, plus
/// the contents of an HTML block, where it does not apply.
#[derive(Clone, Copy, PartialEq)]
enum Context {
    Block,
    Inline,
    HtmlBlock,
}

impl Context {
    fn of_children(self, name: &str) -> Context {
        match self {
            Context::Block => match name {
                "html" | "head" | "body" | "blockquote" | "ol" | "ul" | "li" | "table"
                | "thead" | "tbody" | "tfoot" | "tr" => Context::Block,
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "td" | "th" => Context::Inline,
                _ if is_inline(name) => Context::Inline,
                _ => Context::HtmlBlock,
            },
            other => other,
        }
    }
}

/// The rows of the table which `h0` matches.
struct Walk {
    scripting_enabled: bool,
    rows: [bool; 9],
    /// An element in `<head>` or a `<frameset>`, where the fragment parse of
    /// "Comparison" differs from the document parse of `convert`.
    parsers_disagree: bool,
}

impl Walk {
    /// Parse `h0` as a full document, as `HtmlToMarkdown::html_to_tree` does,
    /// and walk it.
    fn run(h0: &str, scripting_enabled: bool) -> Walk {
        let opts = ParseOpts {
            tree_builder: TreeBuilderOpts {
                scripting_enabled,
                ..Default::default()
            },
            ..Default::default()
        };
        let document = parse_document(RcDom::default(), opts).one(h0).document;
        let mut walk = Walk {
            scripting_enabled,
            rows: [false; 9],
            parsers_disagree: false,
        };
        for child in document.children.borrow().iter() {
            walk.visit(child, Context::Block, false, false, false);
            if is_element(child, "html") {
                walk.rows[7] |= comment_after(child, "body");
            }
        }
        walk.rows[7] |= comment_after(&document, "html");
        walk
    }

    fn visit(
        &mut self,
        node: &Handle,
        context: Context,
        in_type_6_block: bool,
        preformatted: bool,
        in_head: bool,
    ) {
        match &node.data {
            NodeData::Doctype { .. } => self.rows[7] = true,
            NodeData::Comment { contents } => {
                self.rows[2] |= in_type_6_block && has_blank_line(contents);
                self.rows[7] |= in_head;
            }
            NodeData::Text { contents } => {
                let contents = contents.borrow();
                self.rows[6] |= preformatted && contents.contains('\r');
                self.rows[7] |= in_head;
            }
            NodeData::Element { name, attrs, .. } => {
                let name = name.local.as_ref();
                let text = text_content(node);
                match name {
                    "script" | "style" => {
                        self.rows[0] |= context == Context::Inline;
                        self.rows[2] |= in_type_6_block && has_blank_line(&text);
                    }
                    "pre" | "textarea" => {
                        self.rows[1] |= context == Context::Inline
                            && (text.contains(['\t', '\n', '\u{c}', '\r']) || text.contains("  "));
                        if name == "pre" {
                            let mut flat = String::new();
                            flatten(node, false, &mut flat);
                            self.rows[3] |= closes_type_1_then_line_ending(&flat);
                        }
                    }
                    "iframe" => {
                        self.rows[4] |= !text
                            .chars()
                            .all(|ch| ch.is_ascii_alphanumeric() || ch == ' ');
                    }
                    "noembed" | "noframes" | "xmp" | "plaintext" => self.rows[4] = true,
                    "noscript" => self.rows[4] |= self.scripting_enabled,
                    "template" => self.rows[5] = true,
                    "frameset" => {
                        self.rows[7] = true;
                        self.parsers_disagree = true;
                    }
                    "a" | "img" => {
                        self.rows[6] |= attrs.borrow().iter().any(|attr| {
                            matches!(&*attr.name.local, "title" | "alt")
                                && attr.value.contains('\r')
                        });
                    }
                    "span" => {
                        let attrs = attrs.borrow();
                        let children = node.children.borrow();
                        self.rows[8] |= children.len() <= 1
                            && children
                                .iter()
                                .all(|child| matches!(child.data, NodeData::Text { .. }))
                            && attrs.len() == 1
                            && &*attrs[0].name.local == "class"
                            && matches!(&*attrs[0].value, "math math-inline" | "math math-display")
                            && has_unescaped_percent(&text);
                    }
                    _ => {}
                }
                self.rows[7] |= in_head;
                self.parsers_disagree |= in_head;
                let child_context = context.of_children(name);
                let type_6_block = context == Context::Block
                    && child_context == Context::HtmlBlock
                    && TYPE_6.contains(&name);
                for child in node.children.borrow().iter() {
                    self.visit(
                        child,
                        child_context,
                        in_type_6_block || type_6_block,
                        preformatted || matches!(name, "pre" | "textarea"),
                        name == "head",
                    );
                }
            }
            _ => {}
        }
    }
}

fn is_element(node: &Handle, tag: &str) -> bool {
    matches!(&node.data, NodeData::Element { name, .. } if &*name.local == tag)
}

/// Whether a comment among the children of `parent` follows its `tag` child.
fn comment_after(parent: &Handle, tag: &str) -> bool {
    parent
        .children
        .borrow()
        .iter()
        .skip_while(|child| !is_element(child, tag))
        .skip(1)
        .any(|child| matches!(child.data, NodeData::Comment { .. }))
}

/// The text of the descendants of `node`, in document order.
fn text_content(node: &Handle) -> String {
    let mut text = String::new();
    for child in node.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } => text.push_str(&contents.borrow()),
            NodeData::Element { .. } => text.push_str(&text_content(child)),
            _ => {}
        }
    }
    text
}

/// The content of `node` as htmd writes it into an HTML block: escaped text,
/// raw text inside `<script>` and `<style>`, comments, and the end tags of
/// the type 1 elements.
fn flatten(node: &Handle, raw: bool, out: &mut String) {
    for child in node.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } if raw => out.push_str(&contents.borrow()),
            NodeData::Text { contents } => out.push_str(&contents.borrow().replace('<', "&lt;")),
            NodeData::Comment { contents } => out.push_str(&format!("<!--{contents}-->")),
            NodeData::Element { name, .. } => {
                let name = name.local.as_ref();
                flatten(child, matches!(name, "script" | "style"), out);
                if matches!(name, "pre" | "script" | "style" | "textarea") {
                    out.push_str(&format!("</{name}>"));
                }
            }
            _ => {}
        }
    }
}

/// Whether `text` holds a `%` after an even number of backslashes; see "Math".
fn has_unescaped_percent(text: &str) -> bool {
    let mut backslashes = 0;
    for ch in text.chars() {
        if ch == '%' && backslashes % 2 == 0 {
            return true;
        }
        backslashes = if ch == '\\' { backslashes + 1 } else { 0 };
    }
    false
}

/// Whether `text` holds a blank line: a line of only spaces and tabs between
/// two line endings.
fn has_blank_line(text: &str) -> bool {
    let lines: Vec<&str> = text.split(['\n', '\r']).collect();
    lines.len() > 2
        && lines[1..lines.len() - 1]
            .iter()
            .any(|line| line.chars().all(|ch| matches!(ch, ' ' | '\t')))
}

/// Whether `text` holds a type 1 end condition with a line ending after it.
fn closes_type_1_then_line_ending(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    ["</pre>", "</script>", "</style>", "</textarea>"]
        .iter()
        .filter_map(|end| text.find(end).map(|index| index + end.len()))
        .min()
        .is_some_and(|end| text[end..].contains(['\n', '\r']))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(h0: &str) -> Option<usize> {
        let reason = out_of_scope(h0, true)?;
        ROWS.iter().position(|&candidate| candidate == reason)
    }

    #[test]
    fn in_scope() {
        assert_eq!(row("<p>a</p>"), None);
        assert_eq!(row("<div><script>a</script></div>"), None);
        assert_eq!(row("<!--a\n\nb-->"), None);
        assert_eq!(row("<p>a</p><!--a\n\nb-->"), None);
        assert_eq!(row("<pre>a\nb</pre>"), None);
        assert_eq!(row("<p><iframe>a b</iframe></p>"), None);
        assert_eq!(row("<ul><li>a</li></ul>"), None);
    }

    #[test]
    fn script_or_style_inline() {
        // "Type 1 content placed inline": `<h1>a<script>b*c*d</script>e</h1>`.
        assert_eq!(row("<h1>a<script>b*c*d</script>e</h1>"), Some(0));
        assert_eq!(row("<p><em>a<style>b</style></em></p>"), Some(0));
    }

    #[test]
    fn preformatted_inline_with_whitespace() {
        assert_eq!(row("<p><textarea>a\nb</textarea></p>"), Some(1));
        assert_eq!(
            row("<table><tr><td><pre>a  b</pre></td></tr></table>"),
            Some(1)
        );
        assert_eq!(row("<p><textarea>a b</textarea></p>"), None);
    }

    #[test]
    fn blank_line_in_type_6_block() {
        assert_eq!(row("<div><!--a\n\nb--></div>"), Some(2));
        assert_eq!(row("<div><style>a\n \nb</style></div>"), Some(2));
        // A `<blockquote>` is translated, not written as an HTML block.
        assert_eq!(row("<blockquote><!--a\n\nb--></blockquote>"), None);
    }

    #[test]
    fn type_1_end_in_pre() {
        // `<pre><style></style>⏎a</pre>` of "Special case".
        assert_eq!(row("<pre><style></style>\na</pre>"), Some(3));
        assert_eq!(row("<pre><!--</pre>\n--></pre>"), Some(3));
        assert_eq!(row("<pre><!--</PRE>-->\na</pre>"), Some(3));
        assert_eq!(row("<pre><!--</pre>--></pre>"), None);
        assert_eq!(row("<pre>&lt;/pre>\na</pre>"), None);
        assert_eq!(row("<pre>a<pre>b</pre>\nc</pre>"), Some(3));
    }

    #[test]
    fn raw_text_elements() {
        assert_eq!(row("<p><iframe>a-b</iframe></p>"), Some(4));
        assert_eq!(row("<noembed>a</noembed>"), Some(4));
        assert_eq!(row("<xmp>a</xmp>"), Some(4));
        assert_eq!(row("<plaintext>a"), Some(4));
        assert_eq!(row("<noscript>a</noscript>"), Some(4));
        assert_eq!(out_of_scope("<p><noscript>a</noscript></p>", false), None);
    }

    #[test]
    fn template() {
        assert_eq!(row("<template>a</template>"), Some(5));
    }

    #[test]
    fn carriage_return_in_preformatted() {
        assert_eq!(row("<pre>a&#13;b</pre>"), Some(6));
        assert_eq!(row("<textarea>a&#13;b</textarea>"), Some(6));
        assert_eq!(row(r#"<a href="x" title="a&#13;b">t</a>"#), Some(6));
    }

    #[test]
    fn doctype_head_and_frameset() {
        assert_eq!(row("<!DOCTYPE html><p>a</p>"), Some(7));
        assert_eq!(row("<title>a</title>"), Some(7));
        assert_eq!(row("<frameset>a"), Some(7));
        assert_eq!(row("a</body><!--c-->b"), Some(7));
        assert_eq!(row("<!--c-->a"), None);
    }

    #[test]
    fn math_with_percent() {
        assert_eq!(row(r#"<span class="math math-inline">a%b</span>"#), Some(8));
        assert_eq!(
            row(r#"<span class="math math-inline">a\\%b</span>"#),
            Some(8)
        );
        assert_eq!(row(r#"<span class="math math-inline">a\%b</span>"#), None);
        assert_eq!(row(r#"<span class="other">a%b</span>"#), None);
        assert_eq!(
            row(r#"<span class="math math-inline">a%<em>b</em></span>"#),
            None
        );
    }

    #[test]
    fn text_exemptions() {
        assert!(text_exempt("<title>a</title>", true));
        assert!(text_exempt("<frameset>a", true));
        assert!(text_exempt("<h1>a<script>b</script></h1>", true));
        assert!(text_exempt("<xmp>a</xmp>", true));
        assert!(text_exempt("<div><!--a\n\nb--></div>", true));
        assert!(text_exempt(
            r#"<span class="math math-inline">a%b</span>"#,
            true
        ));
        assert!(!text_exempt("<p>a</p>", true));
        assert!(!text_exempt("<p>a</p><template>a</template>", true));
    }
}

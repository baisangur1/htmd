//! "Comparison" and "Normalizer".

use std::collections::BTreeMap;

use html5ever::tendril::TendrilSink;
use html5ever::tree_builder::{QuirksMode, TreeBuilderOpts};
use html5ever::{ParseOpts, QualName, local_name, ns, parse_fragment};
use markup5ever_rcdom::{Handle, NodeData, RcDom};

/// Tag names which start a CommonMark HTML block of type 1.
pub(crate) const TYPE_1: &[&str] = &["pre", "script", "style", "textarea"];

/// Tag names which start a CommonMark HTML block of type 6; see
/// <https://spec.commonmark.org/0.31.2/#html-blocks>.
pub(crate) const TYPE_6: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

/// Whether an element named `name` is an inline child in the sense of
/// "Implied paragraphs".
pub(crate) fn is_inline(name: &str) -> bool {
    !TYPE_1.contains(&name) && !TYPE_6.contains(&name)
}

/// The first node where the two sides differ.
#[derive(Debug)]
pub struct Mismatch {
    /// Path from the fragment root to the input node, such as `/p[0]/em[1]`.
    pub path: String,
    /// The normalized input subtree.
    pub input: String,
    /// The normalized output subtree.
    pub output: String,
}

/// A parsed node, owned so that the normalizer can rewrite it.
#[derive(Clone)]
pub(crate) enum Node {
    Element {
        name: String,
        attrs: BTreeMap<String, String>,
        children: Vec<Node>,
    },
    Text(String),
    Comment(String),
}

/// Match `output_html` against `input_html`.
pub fn compare(
    input_html: &str,
    output_html: &str,
    scripting_enabled: bool,
) -> Result<(), Mismatch> {
    let mut input = parse(input_html, scripting_enabled);
    let mut output = parse(output_html, scripting_enabled);
    normalize(&mut input, "", false);
    normalize(&mut output, "", false);
    compare_children(&input, &output, "", "")
}

/// Parse `html` as a fragment in a `<body>` context.
pub(crate) fn parse(html: &str, scripting_enabled: bool) -> Vec<Node> {
    let opts = ParseOpts {
        tree_builder: TreeBuilderOpts {
            scripting_enabled,
            quirks_mode: QuirksMode::Quirks,
            ..Default::default()
        },
        ..Default::default()
    };
    let context = QualName::new(None, ns!(html), local_name!("body"));
    let dom = parse_fragment(
        RcDom::default(),
        opts,
        context,
        Vec::new(),
        scripting_enabled,
    )
    .one(html);
    // The fragment parser puts the fragment under a single `<html>` element.
    let root = dom.document.children.borrow()[0].clone();
    owned_children(&root)
}

fn owned_children(handle: &Handle) -> Vec<Node> {
    handle
        .children
        .borrow()
        .iter()
        .filter_map(|child| match &child.data {
            NodeData::Text { contents } => Some(Node::Text(contents.borrow().to_string())),
            NodeData::Comment { contents } => Some(Node::Comment(contents.to_string())),
            NodeData::Element { name, attrs, .. } => Some(Node::Element {
                name: name.local.to_string(),
                attrs: attrs
                    .borrow()
                    .iter()
                    .map(|attr| {
                        let name = match &attr.name.prefix {
                            Some(prefix) => format!("{prefix}:{}", attr.name.local),
                            None => attr.name.local.to_string(),
                        };
                        (name, attr.value.to_string())
                    })
                    .collect(),
                children: owned_children(child),
            }),
            _ => None,
        })
        .collect()
}

/// Apply the rules of "Normalizer" to the children of `parent`, which is
/// empty at the fragment root.
fn normalize(children: &mut Vec<Node>, parent: &str, preformatted: bool) {
    for child in children.iter_mut() {
        if let Node::Element {
            name,
            attrs,
            children,
        } = child
        {
            let preformatted =
                preformatted || matches!(name.as_str(), "pre" | "listing" | "textarea");
            normalize(children, name, preformatted);
            normalize_element(name, attrs, children);
        }
    }
    for child in children.iter_mut() {
        match child {
            Node::Comment(data) => *data = collapse_whitespace(data),
            Node::Text(text) if !preformatted => *text = collapse_whitespace(text),
            _ => {}
        }
    }
    if preformatted {
        return;
    }
    if parent.is_empty() || !is_inline(parent) {
        let inline_content = matches!(
            parent,
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "td" | "th" | "caption"
        );
        trim_block_content(children, inline_content);
    }
}

/// Rule 1: whitespace at the start and end of a block's content and, unless
/// that content is inline, next to a child which is not an inline child.
fn trim_block_content(children: &mut Vec<Node>, inline_content: bool) {
    let is_block =
        |node: Option<&Node>| node.is_none_or(|node| !inline_content && !is_inline_child(node));
    for index in 0..children.len() {
        let after_block = is_block(index.checked_sub(1).and_then(|before| children.get(before)));
        let before_block = is_block(children.get(index + 1));
        if let Node::Text(text) = &mut children[index] {
            if after_block {
                *text = text.trim_start_matches(' ').to_string();
            }
            if before_block {
                *text = text.trim_end_matches(' ').to_string();
            }
        }
    }
    children.retain(|child| !matches!(child, Node::Text(text) if text.is_empty()));
}

/// An inline child in the sense of "Implied paragraphs".
fn is_inline_child(node: &Node) -> bool {
    match node {
        Node::Text(_) => true,
        Node::Comment(_) => false,
        Node::Element { name, .. } => is_inline(name),
    }
}

/// Rules 2, 3, 4 and 6, which apply to one element.
fn normalize_element(name: &str, attrs: &mut BTreeMap<String, String>, children: &mut Vec<Node>) {
    match name {
        "pre" => {
            if let [Node::Element { name, children, .. }] = children.as_mut_slice()
                && name == "code"
                && let Some(Node::Text(text)) = children.last_mut()
                && text.ends_with('\n')
            {
                text.pop();
                if text.is_empty() {
                    children.pop();
                }
            }
        }
        "table" => normalize_table(children),
        "ol" => {
            if attrs.get("start").is_some_and(|start| parses_as_one(start)) {
                attrs.remove("start");
            }
        }
        "span" => {
            if attrs.len() == 1
                && attrs
                    .get("class")
                    .is_some_and(|class| class == "math math-inline")
                && let [Node::Text(text)] = children.as_mut_slice()
            {
                *text = text
                    .trim_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r'])
                    .to_string();
            }
        }
        _ => {}
    }
}

/// Rule 3: a leading row of `<th>` cells moves into a `<thead>`, and an empty
/// `<tbody>` after a `<thead>` goes.
fn normalize_table(children: &mut Vec<Node>) {
    if !children.iter().any(|child| is_element(child, "thead"))
        && let Some(index) = children.iter().position(|child| is_element(child, "tbody"))
        && let Node::Element { children: rows, .. } = &mut children[index]
        && let Some(Node::Element {
            children: cells, ..
        }) = rows.first()
        && is_element(&rows[0], "tr")
        && !cells.is_empty()
        && cells.iter().all(|cell| is_element(cell, "th"))
    {
        let row = rows.remove(0);
        children.insert(
            index,
            Node::Element {
                name: "thead".to_string(),
                attrs: BTreeMap::new(),
                children: vec![row],
            },
        );
    }
    let mut index = 1;
    while index < children.len() {
        if is_element(&children[index - 1], "thead")
            && matches!(&children[index], Node::Element { name, children, .. } if name == "tbody" && children.is_empty())
        {
            children.remove(index);
        } else {
            index += 1;
        }
    }
}

/// Whether `value` is 1 by the HTML rules for parsing integers:
/// <https://html.spec.whatwg.org/multipage/common-microsyntaxes.html#rules-for-parsing-integers>.
fn parses_as_one(value: &str) -> bool {
    let value = value.trim_start_matches(['\t', '\n', '\u{c}', '\r', ' ']);
    let value = value.strip_prefix('+').unwrap_or(value);
    let digits = value.bytes().take_while(u8::is_ascii_digit).count();
    value[..digits].parse::<i64>() == Ok(1)
}

fn is_element(node: &Node, tag: &str) -> bool {
    matches!(node, Node::Element { name, .. } if name == tag)
}

/// Replace each run of ASCII whitespace with one space.
fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_run = false;
    for ch in text.chars() {
        if matches!(ch, '\t' | '\n' | '\x0C' | '\r' | ' ') {
            if !in_run {
                out.push(' ');
            }
            in_run = true;
        } else {
            out.push(ch);
            in_run = false;
        }
    }
    out
}

/// Compare the children of `parent`, applying "Implied paragraphs" where it
/// holds.
fn compare_children(
    input: &[Node],
    output: &[Node],
    parent: &str,
    path: &str,
) -> Result<(), Mismatch> {
    let implied_paragraphs = matches!(parent, "" | "blockquote" | "li");
    let mut index = 0;
    let mut output_index = 0;
    while index < input.len() {
        let node_path = format!("{path}/{}[{index}]", label(&input[index]));
        let run_end = if implied_paragraphs {
            index
                + input[index..]
                    .iter()
                    .take_while(|node| is_inline_child(node))
                    .count()
        } else {
            index
        };
        if run_end > index
            && let Some(Node::Element {
                name,
                attrs,
                children,
            }) = output.get(output_index)
            && name == "p"
            && attrs.is_empty()
        {
            compare_children(
                &input[index..run_end],
                children,
                "p",
                &format!("{node_path}/p"),
            )?;
            index = run_end;
            output_index += 1;
            continue;
        }
        // Otherwise the whole run, or the single node, matches as it is.
        let end = run_end.max(index + 1);
        for (offset, node) in input[index..end].iter().enumerate() {
            let node_path = format!("{path}/{}[{}]", label(node), index + offset);
            let Some(output_node) = output.get(output_index) else {
                return Err(Mismatch {
                    path: node_path,
                    input: serialize(&input[index + offset..]),
                    output: String::new(),
                });
            };
            compare_node(node, output_node, &node_path)?;
            output_index += 1;
        }
        index = end;
    }
    if output_index < output.len() {
        return Err(Mismatch {
            path: format!("{path}/[{index}]"),
            input: String::new(),
            output: serialize(&output[output_index..]),
        });
    }
    Ok(())
}

fn compare_node(input: &Node, output: &Node, path: &str) -> Result<(), Mismatch> {
    let matches = match (input, output) {
        (Node::Text(a), Node::Text(b)) | (Node::Comment(a), Node::Comment(b)) => a == b,
        (
            Node::Element {
                name,
                attrs,
                children,
            },
            Node::Element {
                name: output_name,
                attrs: output_attrs,
                children: output_children,
            },
        ) => {
            if name == output_name
                && attrs.len() == output_attrs.len()
                && attrs.iter().all(|(key, value)| {
                    output_attrs.get(key).is_some_and(|output_value| {
                        attribute_matches(name, key, value, output_value)
                    })
                })
            {
                return compare_children(children, output_children, name, path);
            }
            false
        }
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        Err(Mismatch {
            path: path.to_string(),
            input: serialize(std::slice::from_ref(input)),
            output: serialize(std::slice::from_ref(output)),
        })
    }
}

/// "URLs": a destination matches as it is or as pulldown-cmark escapes it.
fn attribute_matches(element: &str, key: &str, input: &str, output: &str) -> bool {
    if input == output {
        return true;
    }
    if !matches!((element, key), ("a", "href") | ("img", "src")) {
        return false;
    }
    let mut escaped = String::new();
    let _ = pulldown_cmark_escape::escape_href(&mut escaped, input);
    escaped.replace("&#x27;", "'").replace("&amp;", "&") == output
}

fn label(node: &Node) -> &str {
    match node {
        Node::Element { name, .. } => name,
        Node::Text(_) => "#text",
        Node::Comment(_) => "#comment",
    }
}

/// Serialize normalized nodes for a failure message.
fn serialize(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(&format!("{text:?}")),
            Node::Comment(data) => out.push_str(&format!("<!--{data:?}-->")),
            Node::Element {
                name,
                attrs,
                children,
            } => {
                out.push_str(&format!("<{name}"));
                for (key, value) in attrs {
                    out.push_str(&format!(" {key}={value:?}"));
                }
                out.push_str(&format!(">{}</{name}>", serialize(children)));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(input: &str, output: &str) -> bool {
        compare(input, output, true).is_ok()
    }

    #[test]
    fn identical_and_different() {
        assert!(matches("<p>a</p>", "<p>a</p>"));
        let mismatch = compare("<p>a</p>", "<p>b</p>", true).unwrap_err();
        assert_eq!(mismatch.path, "/p[0]/#text[0]");
        assert!(!matches("<p>a</p>", "<p>a</p><p>b</p>"));
        assert!(!matches("<p>a</p><p>b</p>", "<p>a</p>"));
    }

    #[test]
    fn attribute_order_is_ignored() {
        assert!(matches(
            r#"<img src="a" alt="b">"#,
            r#"<p><img alt="b" src="a"></p>"#
        ));
    }

    #[test]
    fn whitespace() {
        assert!(matches("<p>a \n\t b</p>", "<p>a b</p>"));
        assert!(matches("<p> a </p>\n<p>b</p>", "<p>a</p><p>b</p>"));
        assert!(matches("<pre>a  b</pre>", "<pre>a  b</pre>"));
        assert!(!matches("<pre>a  b</pre>", "<pre>a b</pre>"));
        assert!(!matches(
            "<listing>a  b</listing>",
            "<p><listing>a b</listing></p>"
        ));
        // "At the document root": `<p>a</p><br>` reads back as `<p>a</p>⏎<br>`.
        assert!(matches("<p>a</p><br>", "<p>a</p>\n<br>"));
        assert!(matches("a <div>b</div>", "<p>a</p>\n<div>b</div>"));
        assert!(!matches("<p>a b</p>", "<p>ab</p>"));
        // In a paragraph the space next to an `<iframe>` shows.
        assert!(!matches(
            "<p>a <iframe>x</iframe> b</p>",
            "<p>a<iframe>x</iframe>b</p>"
        ));
    }

    #[test]
    fn emphasis_whitespace_stays_inside() {
        // The emphasis "Approach" moves it out for Pure mode only.
        assert!(!matches("<p>x<em> a </em>y</p>", "<p>x <em>a</em> y</p>"));
    }

    #[test]
    fn quirks_mode() {
        // In quirks mode `<table>` does not close an open `<p>`.
        assert!(matches(
            "<p>a<table><tr><td>b</td></tr></table></p>",
            "<p>a<table><tbody><tr><td>b</td></tr></tbody></table></p>"
        ));
    }

    #[test]
    fn code_blocks() {
        assert!(matches(
            "<pre><code>a</code></pre>",
            "<pre><code>a\n</code></pre>"
        ));
        assert!(matches(
            "<pre><code>a\n</code></pre>",
            "<pre><code>a\n</code></pre>"
        ));
        assert!(!matches("<pre>a</pre>", "<pre>a\n</pre>"));
    }

    #[test]
    fn table_sections() {
        assert!(matches(
            "<table><tr><th>h</th></tr><tr><td>d</td></tr></table>",
            "<table><thead><tr><th>h</th></tr></thead>\n<tbody>\n<tr><td>d</td></tr>\n</tbody></table>"
        ));
        assert!(matches(
            "<table><tr><th>h</th></tr></table>",
            "<table><thead><tr><th>h</th></tr></thead><tbody></tbody></table>"
        ));
    }

    #[test]
    fn attribute_prefixes() {
        assert!(!matches(
            r#"<svg><a xlink:href="y"></a></svg>"#,
            r#"<p><svg><a href="y"></a></svg></p>"#
        ));
    }

    #[test]
    fn list_start() {
        assert!(matches(
            r#"<ol start="1"><li>a</li></ol>"#,
            "<ol>\n<li>a</li>\n</ol>"
        ));
        assert!(matches(
            r#"<ol start=" 01. A"><li>a</li></ol>"#,
            "<ol>\n<li>a</li>\n</ol>"
        ));
        assert!(!matches(
            r#"<ol start="2"><li>a</li></ol>"#,
            "<ol>\n<li>a</li>\n</ol>"
        ));
    }

    #[test]
    fn comments() {
        assert!(matches("<!--a \n\n b-->", "<!--a b-->"));
    }

    #[test]
    fn math() {
        assert!(matches(
            r#"<p><span class="math math-inline"> a⏎b </span></p>"#
                .replace('⏎', "\n")
                .as_str(),
            r#"<p><span class="math math-inline">a b</span></p>"#
        ));
        assert!(matches(
            "<span class=\"math math-inline\">a&#11;</span>",
            r#"<p><span class="math math-inline">a</span></p>"#
        ));
        assert!(matches(
            r#"<span class="math math-display"> a </span>"#,
            r#"<p><span class="math math-display"> a </span></p>"#
        ));
    }

    #[test]
    fn emphasis_names_stay_distinct() {
        assert!(!matches("<p><i>a</i></p>", "<p><em>a</em></p>"));
        assert!(!matches("<p><b>a</b></p>", "<p><strong>a</strong></p>"));
    }

    #[test]
    fn implied_paragraphs() {
        // "At the document root": `a<em>b</em>` reads back inside a `<p>`.
        assert!(matches("a<em>b</em>", "<p>a<em>b</em></p>\n"));
        assert!(matches(
            "<blockquote>a</blockquote>",
            "<blockquote>\n<p>a</p>\n</blockquote>"
        ));
        assert!(matches("<ul><li>a</li></ul>", "<ul>\n<li>a</li>\n</ul>"));
        // A `<p>` on the input side matches only a `<p>`.
        assert!(!matches("<p><br></p>", "<br>"));
        // The rule runs one way only.
        assert!(!matches("<p>a</p>", "a"));
        // A comment is not an inline child: `a<!--c-->b` gives `a⏎⏎<!--c-->⏎⏎b`.
        assert!(matches("a<!--c-->b", "<p>a</p>\n<!--c-->\n<p>b</p>"));
        // Nor does it apply inside other elements.
        assert!(!matches("<div>a</div>", "<div><p>a</p></div>"));
        // Only the whole run is wrapped.
        assert!(!matches("a<em>b</em>", "a<p><em>b</em></p>"));
    }

    #[test]
    fn urls() {
        assert!(matches(
            r#"<a href="a b&amp;c'ä">x</a>"#,
            r#"<p><a href="a%20b&amp;c&#x27;%C3%A4">x</a></p>"#
        ));
        assert!(matches(r#"<img src="a b">"#, r#"<p><img src="a%20b"></p>"#));
        assert!(!matches(
            r#"<a title="a b">x</a>"#,
            r#"<p><a title="a%20b">x</a></p>"#
        ));
        assert!(!matches(
            r#"<a href="&amp;#x27;">x</a>"#,
            r#"<p><a href="'">x</a></p>"#
        ));
        // The rule runs one way only.
        assert!(!matches(
            r#"<a href="a%20b">x</a>"#,
            r#"<p><a href="a b">x</a></p>"#
        ));
    }
}

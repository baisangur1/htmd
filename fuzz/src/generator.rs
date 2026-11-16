//! The generator of `round_trip_structured`; see "Targets >
//! round_trip_structured".

/// Deepest element nesting the generator builds.
const MAX_DEPTH: usize = 8;
/// Most children an element gets.
const MAX_CHILDREN: usize = 6;

/// The elements with a built-in handler, except the document-level `head`,
/// `body` and `html`, then the math spans, then the elements without a
/// dedicated handler.
const TAGS: &[Tag] = &[
    Tag { name: "a", math_class: None, void_: false },
    Tag { name: "b", math_class: None, void_: false },
    Tag { name: "blockquote", math_class: None, void_: false },
    Tag { name: "br", math_class: None, void_: true },
    Tag { name: "caption", math_class: None, void_: false },
    Tag { name: "code", math_class: None, void_: false },
    Tag { name: "em", math_class: None, void_: false },
    Tag { name: "h1", math_class: None, void_: false },
    Tag { name: "h2", math_class: None, void_: false },
    Tag { name: "h3", math_class: None, void_: false },
    Tag { name: "h4", math_class: None, void_: false },
    Tag { name: "h5", math_class: None, void_: false },
    Tag { name: "h6", math_class: None, void_: false },
    Tag { name: "hr", math_class: None, void_: true },
    Tag { name: "i", math_class: None, void_: false },
    Tag { name: "img", math_class: None, void_: true },
    Tag { name: "li", math_class: None, void_: false },
    Tag { name: "ol", math_class: None, void_: false },
    Tag { name: "p", math_class: None, void_: false },
    Tag { name: "pre", math_class: None, void_: false },
    Tag { name: "strong", math_class: None, void_: false },
    Tag { name: "table", math_class: None, void_: false },
    Tag { name: "tbody", math_class: None, void_: false },
    Tag { name: "td", math_class: None, void_: false },
    Tag { name: "th", math_class: None, void_: false },
    Tag { name: "thead", math_class: None, void_: false },
    Tag { name: "tr", math_class: None, void_: false },
    Tag { name: "ul", math_class: None, void_: false },
    Tag { name: "span", math_class: None, void_: false },
    Tag { name: "span", math_class: Some("math math-inline"), void_: false },
    Tag { name: "span", math_class: Some("math math-display"), void_: false },
    Tag { name: "div", math_class: None, void_: false },
    Tag { name: "del", math_class: None, void_: false },
    Tag { name: "sub", math_class: None, void_: false },
    Tag { name: "sup", math_class: None, void_: false },
    Tag { name: "kbd", math_class: None, void_: false },
    Tag { name: "colgroup", math_class: None, void_: false },
    Tag { name: "col", math_class: None, void_: true },
    Tag { name: "tfoot", math_class: None, void_: false },
];

/// An element the generator can emit.
#[derive(Clone, Copy)]
struct Tag {
    name: &'static str,
    math_class: Option<&'static str>,
    void_: bool,
}

/// The attributes the generator emits, present or absent, empty or not.
const ATTRIBUTES: &[&str] = &[
    "href", "src", "alt", "title", "class", "id", "start", "align",
];

/// The characters which are CommonMark syntax somewhere.
const SYNTAX_CHARS: &[&str] = &[
    "\\", "`", "*", "_", "[", "]", "(", ")", "<", ">", "&", ";", "!", "|", "$",
    "#", "~", "-", "+", "=", ".", ":", "'", "\"", "%", "{", "}",
];

/// The other characters text is drawn from: digits, `a` and `A`, whitespace,
/// look-alike whitespace and multi-byte characters.
const OTHER_CHARS: &[&str] = &[
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "a", "A", " ", "\t",
    "\n", "\r", "\u{c}", "\u{a0}", "\u{2003}", "\u{3000}", "¡", "\u{200b}",
    "\u{2028}", "\u{feff}", "é", "中", "😀",
];

/// The words inserted among the text characters.
const WORDS: &[&str] = &["lt", "amp", "copy", "div", "pre", "CDATA"];

/// The CommonMark block syntax a text node can start with.
const SEQUENCES: &[&str] = &[
    "1. A", "1) A", "* a", "- a", "+ a", "---", "***", "___", "===", "# a",
    "> a", "```", "~~~", "    ", "[a]: b", "| a |",
];

/// Build HTML from the generator's choices.
///
/// Deterministic for the same bytes; never panics.
pub fn generate(choices: &[u8]) -> String {
    let Some(&first) = choices.first() else {
        return String::new();
    };
    let nodes = build_tree(&choices[1..]);
    let mut out = String::new();
    match first % 4 {
        // Half of the trees sit between neighbors which stay the same.
        0 => {
            out.push_str("<p>before</p>");
            serialize_nodes(&nodes, &mut out);
            out.push_str("<p>after</p>");
        }
        2 => {
            out.push_str("<p>b<em>e</em>");
            serialize_nodes(&nodes, &mut out);
            out.push_str("<em>a</em>f</p>");
        }
        _ => serialize_nodes(&nodes, &mut out),
    }
    out
}

/// Build the tree the choices describe, without the leading neighbor byte.
fn build_tree(choices: &[u8]) -> Vec<Node> {
    let mut cursor = Generator {
        choices,
        pos: 0,
        word: 0,
    };
    cursor.build_root()
}

/// A node of the generated tree.
enum Node {
    Text(String),
    Comment(String),
    Element {
        tag: &'static str,
        attrs: Vec<(&'static str, String)>,
        children: Vec<Node>,
    },
    /// Markup which does not nest, serialized as written.
    Raw(String),
}

/// The byte cursor which turns choices into a tree.
struct Generator<'a> {
    choices: &'a [u8],
    pos: usize,
    /// Number of `w0`, `w1`, ... words inserted so far.
    word: usize,
}

impl Generator<'_> {
    fn next(&mut self) -> Option<u8> {
        let byte = *self.choices.get(self.pos)?;
        self.pos += 1;
        Some(byte)
    }

    fn build_root(&mut self) -> Vec<Node> {
        let count = match self.next() {
            Some(byte) => 1 + byte as usize % MAX_CHILDREN,
            None => return Vec::new(),
        };
        let mut nodes = Vec::new();
        for _ in 0..count {
            match self.build_node(0, None, false) {
                Some(node) => nodes.push(node),
                None => break,
            }
        }
        nodes
    }

    fn build_node(
        &mut self,
        depth: usize,
        parent: Option<&'static str>,
        in_table: bool,
    ) -> Option<Node> {
        let kind = self.next()?;
        // One node in about twelve is replaced by markup which does not nest.
        if kind % 12 == 0 {
            return Some(self.build_broken(depth, parent, in_table));
        }
        match kind % 12 {
            1..=4 => self.build_text(),
            5 => self.build_comment(),
            _ => self.build_element(depth, in_table),
        }
    }

    /// A stray end tag, an unclosed start tag, a `<td>` outside a table, or
    /// text directly inside a `<table>` or `<tr>`.
    fn build_broken(&mut self, depth: usize, parent: Option<&'static str>, in_table: bool) -> Node {
        let Some(byte) = self.next() else {
            return Node::Raw("</p>".to_string());
        };
        let stray_end_tag = if byte % 8 < 4 { "</p>" } else { "</li>" };
        match byte % 4 {
            0 => Node::Raw(stray_end_tag.to_string()),
            1 => Node::Raw(self.build_unclosed_start()),
            2 if !in_table => self.build_td(depth),
            3 if matches!(parent, Some("table") | Some("tr")) => self
                .build_text()
                .unwrap_or_else(|| Node::Text(String::new())),
            _ => Node::Raw(stray_end_tag.to_string()),
        }
    }

    /// A start tag with attributes but no end tag.
    fn build_unclosed_start(&mut self) -> String {
        let byte = match self.next() {
            Some(byte) => byte,
            None => return "<p>".to_string(),
        };
        let mut index = byte as usize % TAGS.len();
        while TAGS[index].void_ {
            index = (index + 1) % TAGS.len();
        }
        let tag = TAGS[index];
        let attrs = self.build_attrs(tag.math_class);
        let mut out = String::new();
        push_start_tag(tag.name, &attrs, &mut out);
        out
    }

    fn build_td(&mut self, depth: usize) -> Node {
        // A `<td>` at the deepest level would nest one level too far.
        if depth >= MAX_DEPTH {
            return self
                .build_text()
                .unwrap_or_else(|| Node::Text(String::new()));
        }
        let attrs = self.build_attrs(None);
        let mut children = Vec::new();
        if depth + 1 < MAX_DEPTH {
            let count = match self.next() {
                Some(byte) => byte as usize % (MAX_CHILDREN + 1),
                None => 0,
            };
            for _ in 0..count {
                match self.build_node(depth + 1, Some("td"), true) {
                    Some(node) => children.push(node),
                    None => break,
                }
            }
        }
        Node::Element {
            tag: "td",
            attrs,
            children,
        }
    }

    fn build_element(&mut self, depth: usize, in_table: bool) -> Option<Node> {
        if depth >= MAX_DEPTH {
            return self.build_text();
        }
        let byte = self.next()?;
        let tag = TAGS[byte as usize % TAGS.len()];
        let attrs = self.build_attrs(tag.math_class);
        let mut children = Vec::new();
        if !tag.void_ {
            let count = match self.next() {
                Some(byte) => byte as usize % (MAX_CHILDREN + 1),
                None => 0,
            };
            let children_in_table = in_table || opens_table(tag.name);
            for _ in 0..count {
                match self.build_node(depth + 1, Some(tag.name), children_in_table) {
                    Some(node) => children.push(node),
                    None => break,
                }
            }
        }
        Some(Node::Element {
            tag: tag.name,
            attrs,
            children,
        })
    }

    fn build_attrs(&mut self, math_class: Option<&'static str>) -> Vec<(&'static str, String)> {
        // A math span holds that class and nothing else.
        if let Some(class) = math_class {
            return vec![("class", class.to_string())];
        }
        let mut attrs = Vec::new();
        for &name in ATTRIBUTES {
            let Some(byte) = self.next() else {
                break;
            };
            if byte < 85 {
                continue;
            }
            if byte < 170 {
                attrs.push((name, String::new()));
                continue;
            }
            let mut value = self.build_value();
            // A `class` may also be `language-a`, alone or after another class.
            if name == "class"
                && let Some(extra) = self.next()
                && extra % 4 == 0
            {
                value = if extra % 8 == 0 {
                    format!("{value} language-a")
                } else {
                    "language-a".to_string()
                };
            }
            attrs.push((name, value));
        }
        attrs
    }

    fn build_value(&mut self) -> String {
        let length = match self.next() {
            Some(byte) => 1 + byte as usize % 8,
            None => return String::new(),
        };
        let mut value = String::new();
        for _ in 0..length {
            match self.next() {
                Some(byte) => value.push_str(pick_char(byte)),
                None => break,
            }
        }
        value
    }

    fn build_comment(&mut self) -> Option<Node> {
        let length = 1 + self.next()? as usize % 6;
        let mut data = String::new();
        for _ in 0..length {
            match self.next() {
                Some(byte) => {
                    let ch = pick_char(byte);
                    // `--` would end the comment early, and `<!-->` is not a
                    // comment at all.
                    if ch != "-" && !(data.is_empty() && ch == ">") {
                        data.push_str(ch);
                    }
                }
                None => break,
            }
        }
        Some(Node::Comment(data))
    }

    fn build_text(&mut self) -> Option<Node> {
        let length = 1 + self.next()? as usize % 8;
        let mut text = String::new();
        // The block syntax sequences are placed at the start of every text
        // node, since htmd escapes a text node's first byte without knowing
        // whether the node starts a line.
        if let Some(byte) = self.next() {
            text.push_str(SEQUENCES[byte as usize % SEQUENCES.len()]);
        }
        for _ in 0..length {
            let Some(byte) = self.next() else {
                break;
            };
            match byte % 8 {
                6 => {
                    text.push_str(&format!("w{}", self.word));
                    self.word += 1;
                }
                7 => text.push_str(WORDS[byte as usize / 8 % WORDS.len()]),
                _ => {
                    let ch = pick_char(byte);
                    text.push_str(ch);
                    // A CR followed by LF is one line ending. The byte after
                    // the CR selects the sequence, and the LF follows it, so
                    // both are consumed before the sequence is inserted.
                    let mut after_crlf = false;
                    if ch == "\r"
                        && let Some(&selector) = self.choices.get(self.pos)
                        && let Some(&next) = self.choices.get(self.pos + 1)
                        && next % 8 < 6
                        && pick_char(next) == "\n"
                    {
                        self.pos += 2;
                        text.push('\n');
                        text.push_str(SEQUENCES[selector as usize % SEQUENCES.len()]);
                        after_crlf = true;
                    }
                    // Also right after a line ending in the same node.
                    if !after_crlf
                        && matches!(ch, "\n" | "\r")
                        && let Some(after) = self.next()
                    {
                        text.push_str(SEQUENCES[after as usize % SEQUENCES.len()]);
                    }
                }
            }
        }
        Some(Node::Text(text))
    }
}

/// The text alphabet, mostly CommonMark syntax characters.
fn pick_char(byte: u8) -> &'static str {
    if byte < 192 {
        SYNTAX_CHARS[byte as usize % SYNTAX_CHARS.len()]
    } else {
        OTHER_CHARS[(byte as usize - 192) % OTHER_CHARS.len()]
    }
}

fn serialize_nodes(nodes: &[Node], out: &mut String) {
    for node in nodes {
        serialize_node(node, out);
    }
}

fn serialize_node(node: &Node, out: &mut String) {
    match node {
        Node::Text(text) => escape_text(text, out),
        Node::Comment(data) => {
            out.push_str("<!--");
            out.push_str(data);
            out.push_str("-->");
        }
        Node::Element {
            tag,
            attrs,
            children,
        } => {
            push_start_tag(tag, attrs, out);
            if !is_void(tag) {
                serialize_nodes(children, out);
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
        }
        Node::Raw(raw) => out.push_str(raw),
    }
}

fn push_start_tag(tag: &str, attrs: &[(&'static str, String)], out: &mut String) {
    out.push('<');
    out.push_str(tag);
    for (name, value) in attrs {
        out.push(' ');
        out.push_str(name);
        out.push_str("=\"");
        escape_attr(value, out);
        out.push('"');
    }
    out.push('>');
}

fn is_void(tag: &str) -> bool {
    matches!(tag, "br" | "col" | "hr" | "img")
}

/// Whether `tag` is a table part whose descendants are still in the table,
/// where a `<td>` is a valid cell rather than a stray start tag.
fn opens_table(tag: &str) -> bool {
    matches!(tag, "table" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th")
}

fn escape_text(text: &str, out: &mut String) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            _ => out.push(ch),
        }
    }
}

fn escape_attr(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            '\n' => out.push_str("&#10;"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(generate(b""), "");
    }

    #[test]
    fn deterministic() {
        let lcg = |state: &mut u64| {
            *state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (*state >> 33) as u8
        };
        for seed in 0..64u64 {
            let mut state = seed;
            let choices: Vec<u8> = (0..257).map(|_| lcg(&mut state)).collect();
            assert_eq!(generate(&choices), generate(&choices));
        }
    }

    #[test]
    fn depth_and_child_limits() {
        // Every level picks an element (kind 11), a tag, eight absent
        // attributes and one child.
        let mut choices = vec![0u8];
        let level = [0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x01];
        for _ in 0..40 {
            choices.extend_from_slice(&level);
        }
        let tree = build_tree(&choices);
        assert_eq!(tree.len(), 1);
        assert!(within_limits(&tree));
        assert_eq!(max_depth(&tree), MAX_DEPTH);

        // A broken `<td>` at the deepest level must not add a ninth level.
        let mut broken = vec![0u8];
        for _ in 0..MAX_DEPTH {
            broken.extend_from_slice(&level);
        }
        broken.extend_from_slice(&[0, 2, 0, 0]);
        let tree = build_tree(&broken);
        assert!(within_limits(&tree));
        assert_eq!(max_depth(&tree), MAX_DEPTH);
    }

    #[test]
    fn six_children() {
        // One root element with six broken children.
        let choices = [
            1u8, 0x0B, 0x0B, 0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let tree = build_tree(&choices);
        match &tree[0] {
            Node::Element { children, .. } => assert_eq!(children.len(), MAX_CHILDREN),
            _ => panic!("expected an element"),
        }
    }

    fn max_depth(nodes: &[Node]) -> usize {
        nodes
            .iter()
            .map(|node| match node {
                Node::Element { children, .. } => 1 + max_depth(children),
                _ => 0,
            })
            .max()
            .unwrap_or(0)
    }

    fn within_limits(nodes: &[Node]) -> bool {
        nodes.iter().all(|node| match node {
            Node::Element { children, .. } => {
                children.len() <= MAX_CHILDREN && within_limits(children)
            }
            _ => true,
        })
    }

    #[test]
    fn never_panics() {
        for seed in 0..64u64 {
            let mut state = seed;
            let mut choices = Vec::new();
            for _ in 0..257 {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                choices.push((state >> 33) as u8);
            }
            let _ = generate(&choices);
        }
        for byte in 0..=255u8 {
            let _ = generate(&[byte]);
            let _ = generate(&[byte; 64]);
        }
    }
}


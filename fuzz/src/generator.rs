//! The generator of `round_trip_structured`; see "Targets >
//! round_trip_structured".

/// Deepest element nesting the generator builds.
const MAX_DEPTH: usize = 8;
/// Most children an element gets.
const MAX_CHILDREN: usize = 6;

/// The elements with a built-in handler, except the document-level `head`,
/// `body` and `html`, then the math spans, then the elements without a
/// dedicated handler; each with the class a math span requires.
const TAGS: &[(&str, Option<&str>)] = &[
    ("a", None),
    ("b", None),
    ("blockquote", None),
    ("br", None),
    ("caption", None),
    ("code", None),
    ("em", None),
    ("h1", None),
    ("h2", None),
    ("h3", None),
    ("h4", None),
    ("h5", None),
    ("h6", None),
    ("hr", None),
    ("i", None),
    ("img", None),
    ("li", None),
    ("ol", None),
    ("p", None),
    ("pre", None),
    ("strong", None),
    ("table", None),
    ("tbody", None),
    ("td", None),
    ("th", None),
    ("thead", None),
    ("tr", None),
    ("ul", None),
    ("span", None),
    ("span", Some("math math-inline")),
    ("span", Some("math math-display")),
    ("div", None),
    ("del", None),
    ("sub", None),
    ("sup", None),
    ("kbd", None),
    ("colgroup", None),
    ("col", None),
    ("tfoot", None),
];

/// The attributes the generator emits, present or absent, empty or not.
const ATTRIBUTES: &[&str] = &[
    "href", "src", "alt", "title", "class", "id", "start", "align",
];

/// The characters which are CommonMark syntax somewhere.
const SYNTAX_CHARS: &[&str] = &[
    "\\", "`", "*", "_", "[", "]", "(", ")", "<", ">", "&", ";", "!", "|", "$", "#", "~", "-", "+",
    "=", ".", ":", "'", "\"", "%", "{", "}",
];

/// The other characters text is drawn from: digits, `a` and `A`, whitespace,
/// look-alike whitespace and multi-byte characters.
const OTHER_CHARS: &[&str] = &[
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "a", "A", " ", "\t", "\n", "\r", "\u{c}",
    "\u{a0}", "\u{2003}", "\u{3000}", "¡", "\u{200b}", "\u{2028}", "\u{feff}", "é", "中", "😀",
];

/// The words inserted among the text characters.
const WORDS: &[&str] = &["lt", "amp", "copy", "div", "pre", "CDATA"];

/// The CommonMark block syntax a text node can start with.
const SEQUENCES: &[&str] = &[
    "1. A", "1) A", "* a", "- a", "+ a", "---", "***", "___", "===", "# a", "> a", "```", "~~~",
    "    ", "[a]: b", "| a |",
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
        let kind = self.next()? % 12;
        match kind {
            // One node in about twelve is replaced by markup which does not nest.
            0 => self.build_broken(depth, parent, in_table),
            1..=4 => self.build_text(),
            5 => self.build_comment(),
            _ => {
                let (tag, math_class) = TAGS[self.next()? as usize % TAGS.len()];
                self.build_element(tag, math_class, depth, in_table)
            }
        }
    }

    /// A stray end tag, an unclosed start tag, a `<td>` outside a table, or
    /// text directly inside a `<table>` or `<tr>`.
    fn build_broken(
        &mut self,
        depth: usize,
        parent: Option<&'static str>,
        in_table: bool,
    ) -> Option<Node> {
        let byte = self.next()?;
        let stray_end_tag = if byte % 8 < 4 { "</p>" } else { "</li>" };
        match byte % 4 {
            1 => {
                let mut index = self.next()? as usize % TAGS.len();
                while is_void(TAGS[index].0) {
                    index = (index + 1) % TAGS.len();
                }
                let (tag, math_class) = TAGS[index];
                let attrs = self.build_attrs(math_class);
                let mut out = String::new();
                push_start_tag(tag, &attrs, &mut out);
                Some(Node::Raw(out))
            }
            2 if !in_table => self.build_element("td", None, depth, false),
            3 if matches!(parent, Some("table" | "tr")) => self.build_text(),
            _ => Some(Node::Raw(stray_end_tag.to_string())),
        }
    }

    fn build_element(
        &mut self,
        tag: &'static str,
        math_class: Option<&'static str>,
        depth: usize,
        in_table: bool,
    ) -> Option<Node> {
        if depth >= MAX_DEPTH {
            return self.build_text();
        }
        let attrs = self.build_attrs(math_class);
        let mut children = Vec::new();
        if !is_void(tag) {
            let count = self
                .next()
                .map_or(0, |byte| byte as usize % (MAX_CHILDREN + 1));
            let children_in_table = in_table || opens_table(tag);
            for _ in 0..count {
                match self.build_node(depth + 1, Some(tag), children_in_table) {
                    Some(node) => children.push(node),
                    None => break,
                }
            }
        }
        Some(Node::Element {
            tag,
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
            // A `class` may also be `language-a`, alone or after another
            // class, which a code block writes as its info string.
            if name == "class" {
                let selector = self.next();
                if let Some(extra) = selector
                    && extra % 4 == 0
                {
                    value = if extra % 8 == 0 {
                        format!("{value} language-a")
                    } else {
                        "language-a".to_string()
                    };
                }
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
                    text.push_str(&format!("w{}x", self.word));
                    self.word += 1;
                }
                7 => text.push_str(WORDS[byte as usize / 8 % WORDS.len()]),
                _ => {
                    let ch = pick_char(byte);
                    text.push_str(ch);
                    // Also right after a line ending, which a CR may extend to CRLF.
                    if matches!(ch, "\n" | "\r")
                        && let Some(after) = self.next()
                    {
                        if ch == "\r" && after % 2 == 1 {
                            text.push('\n');
                        }
                        text.push_str(SEQUENCES[after as usize / 2 % SEQUENCES.len()]);
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
    matches!(
        tag,
        "table" | "thead" | "tbody" | "tfoot" | "tr" | "td" | "th"
    )
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

    /// 257 pseudo-random choice bytes from an LCG.
    fn random_choices(seed: u64) -> Vec<u8> {
        let mut state = seed;
        (0..257)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (state >> 33) as u8
            })
            .collect()
    }

    #[test]
    fn deterministic() {
        for seed in 0..64 {
            let choices = random_choices(seed);
            assert_eq!(generate(&choices), generate(&choices));
        }
    }

    #[test]
    fn depth_and_child_limits() {
        // Every level picks an element (kind 11), a tag, eight absent
        // attributes and one child.
        let mut choices = vec![0u8];
        let level = [
            0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x01,
        ];
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
        // Two root nodes: an element with six broken children, then nothing,
        // since the input runs out.
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
        for seed in 0..64 {
            generate(&random_choices(seed));
        }
        for byte in 0..=255u8 {
            let _ = generate(&[byte]);
            let _ = generate(&[byte; 64]);
        }
    }
}

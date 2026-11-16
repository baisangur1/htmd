//! "Text".

use crate::compare::{Node, parse};
use crate::scope::text_exempt;

/// The text `H0` loses.
#[derive(Debug)]
pub struct LostText {
    /// The first numbered word (`w0`, `w1`, ...) of `H0` missing from the
    /// rendered side, if any.
    pub word: Option<String>,
    /// The letters and digits of `H0`.
    pub input: String,
    /// The letters and digits of the rendered side.
    pub output: String,
}

/// Check that `rendered` keeps the letters and digits of `h0`.
pub fn check_text(h0: &str, rendered: &str, scripting_enabled: bool) -> Result<(), LostText> {
    let input = alphanumerics(&parse(h0, scripting_enabled));
    let output = alphanumerics(&parse(rendered, scripting_enabled));
    if input == output || text_exempt(h0, scripting_enabled) {
        return Ok(());
    }
    let output_words = numbered_words(&output);
    let word = numbered_words(&input)
        .into_iter()
        .find(|word| !output_words.contains(word))
        .map(str::to_string);
    Err(LostText {
        word,
        input,
        output,
    })
}

fn alphanumerics(nodes: &[Node]) -> String {
    let mut out = String::new();
    for node in nodes {
        match node {
            Node::Text(text) => out.extend(text.chars().filter(|ch| ch.is_alphanumeric())),
            Node::Element { children, .. } => out.push_str(&alphanumerics(children)),
            Node::Comment(_) => {}
        }
    }
    out
}

/// The `w` + digits words of `text`.
fn numbered_words(text: &str) -> Vec<&str> {
    let mut words = Vec::new();
    for (start, _) in text.match_indices('w') {
        let digits = text[start + 1..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        if digits > 0 {
            words.push(&text[start..start + 1 + digits]);
        }
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kept_text() {
        assert!(check_text("a<em>b</em>", "<p>a<em>b</em></p>\n", true).is_ok());
        // Punctuation and whitespace are left out.
        assert!(check_text("<p>a * b</p>", "<p>a*b</p>", true).is_ok());
        // Attribute values and comments are not text.
        assert!(
            check_text(
                r#"<a href="x">y</a><!--z-->"#,
                "<p><a href=\"u\">y</a></p>",
                true
            )
            .is_ok()
        );
    }

    #[test]
    fn lost_text_names_the_numbered_word() {
        let lost = check_text("<p>a w0 b w1 c w12</p>", "<p>a w0 b c w12</p>", true).unwrap_err();
        assert_eq!(lost.word.as_deref(), Some("w1"));
        let lost = check_text("<p>ab</p>", "<p>a</p>", true).unwrap_err();
        assert_eq!(lost.word, None);
    }

    #[test]
    fn exemptions() {
        assert!(check_text("<title>a</title>", "", true).is_ok());
        // "Math": `a%b` becomes `a`.
        assert!(
            check_text(
                r#"<span class="math math-inline">a%b</span>"#,
                "<p>a</p>",
                true
            )
            .is_ok()
        );
    }
}

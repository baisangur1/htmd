//! "Comparison" and "Normalizer".

/// Path to the differing node and both sides.
#[derive(Debug)]
pub struct Mismatch {}

/// Match `output_html` against `input_html`.
pub fn compare(
    input_html: &str,
    output_html: &str,
    scripting_enabled: bool,
) -> Result<(), Mismatch> {
    todo!()
}

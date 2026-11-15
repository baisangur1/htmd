//! Decoding of a fuzz input; see "Input decoding".

use htmd::options::Options;

/// Which fields of `Options` a target lets the input choose.
pub enum Target {
    Convert,
    RoundTrip,
}

/// Options and HTML decoded from a fuzz input.
pub struct FuzzInput<'a> {
    pub options: Options,
    pub scripting_enabled: bool,
    pub html_bytes: &'a [u8],
}

/// Decode `data`, or `None` if it is rejected.
pub fn decode(data: &[u8], target: Target) -> Option<FuzzInput<'_>> {
    todo!()
}

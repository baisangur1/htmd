//! The round-trip oracle shared by `round_trip` and `round_trip_structured`;
//! steps 1-5 of "Targets > round_trip".

use htmd::HtmlToMarkdown;
use htmd::options::Options;

use crate::compare::compare;
use crate::render;
use crate::scope::out_of_scope;
use crate::text::check_text;

/// Run the oracle on `h0`, or return the failure message.
pub fn round_trip(h0: &str, options: Options, scripting_enabled: bool) -> Result<(), String> {
    let converter = HtmlToMarkdown::builder()
        .options(options)
        .scripting_enabled(scripting_enabled)
        .build();
    let m1 = converter
        .convert(h0)
        .map_err(|error| format!("convert returned Err: {error:?}"))?;
    let h1 = render(&m1);
    if out_of_scope(h0, scripting_enabled).is_some() {
        if let Err(lost) = check_text(h0, &h1, scripting_enabled) {
            return Err(format!("lost text: {lost:?}"));
        }
        return Ok(());
    }
    if let Err(mismatch) = compare(h0, &h1, scripting_enabled) {
        return Err(format!("H1 does not match H0: {mismatch:?}\nM1: {m1:?}"));
    }
    let m2 = converter
        .convert(&h1)
        .map_err(|error| format!("convert returned Err: {error:?}"))?;
    if let Err(mismatch) = compare(&h1, &render(&m2), scripting_enabled) {
        return Err(format!("H2 does not match H1: {mismatch:?}\nM2: {m2:?}"));
    }
    Ok(())
}

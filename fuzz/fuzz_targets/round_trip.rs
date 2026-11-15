#![no_main]

use htmd::HtmlToMarkdown;
use htmd_fuzz::compare::compare;
use htmd_fuzz::options::{Target, decode};
use htmd_fuzz::render;
use htmd_fuzz::scope::out_of_scope;
use htmd_fuzz::text::check_text;
use libfuzzer_sys::{Corpus, fuzz_target};

fuzz_target!(|data: &[u8]| -> Corpus {
    let Some(input) = decode(data, Target::RoundTrip) else {
        return Corpus::Reject;
    };
    let Ok(h0) = str::from_utf8(input.html_bytes) else {
        return Corpus::Reject;
    };
    let scripting_enabled = input.scripting_enabled;
    let converter = HtmlToMarkdown::builder()
        .options(input.options)
        .scripting_enabled(scripting_enabled)
        .build();
    let m1 = converter.convert(h0).expect("convert returned Err");
    let h1 = render(&m1);
    if out_of_scope(h0, scripting_enabled).is_some() {
        check_text(h0, &h1, scripting_enabled).expect("lost text");
        return Corpus::Keep;
    }
    if let Err(mismatch) = compare(h0, &h1, scripting_enabled) {
        panic!("H1 does not match H0: {mismatch:?}\nM1: {m1:?}");
    }
    let m2 = converter.convert(&h1).expect("convert returned Err");
    if let Err(mismatch) = compare(&h1, &render(&m2), scripting_enabled) {
        panic!("H2 does not match H1: {mismatch:?}\nM2: {m2:?}");
    }
    Corpus::Keep
});

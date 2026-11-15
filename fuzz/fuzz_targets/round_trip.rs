#![no_main]

use htmd_fuzz::options::{Target, decode};
use htmd_fuzz::oracle;
use libfuzzer_sys::{Corpus, fuzz_target};

fuzz_target!(|data: &[u8]| -> Corpus {
    let Some(input) = decode(data, Target::RoundTrip) else {
        return Corpus::Reject;
    };
    let Ok(h0) = str::from_utf8(input.html_bytes) else {
        return Corpus::Reject;
    };
    match oracle::round_trip(h0, input.options, input.scripting_enabled) {
        Ok(()) => Corpus::Keep,
        Err(message) => panic!("{message}"),
    }
});

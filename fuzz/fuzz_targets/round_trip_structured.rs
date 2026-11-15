#![no_main]

use std::sync::atomic::{AtomicU64, Ordering};

use htmd_fuzz::generator::generate;
use htmd_fuzz::oracle;
use htmd_fuzz::options::{Target, decode};
use htmd_fuzz::scope::out_of_scope;
use libfuzzer_sys::{Corpus, fuzz_target};

/// Inputs the scope check rejected since the run started.
static REJECTED: AtomicU64 = AtomicU64::new(0);
/// Inputs executed since the run started.
static EXECUTED: AtomicU64 = AtomicU64::new(0);

fuzz_target!(|data: &[u8]| -> Corpus {
    let Some(input) = decode(data, Target::RoundTrip) else {
        return Corpus::Reject;
    };
    let html = generate(input.html_bytes);
    if out_of_scope(&html, input.scripting_enabled).is_some() {
        REJECTED.fetch_add(1, Ordering::Relaxed);
    }
    let executed = EXECUTED.fetch_add(1, Ordering::Relaxed) + 1;
    // libFuzzer has no user-defined statistic, so the count is printed in its
    // `stat::` format; it appears with the statistics of `-print_final_stats=1`.
    if executed % 65536 == 0 {
        eprintln!(
            "stat::round_trip_structured_scope_rejected: {} of {}",
            REJECTED.load(Ordering::Relaxed),
            executed
        );
    }
    match oracle::round_trip(&html, input.options, input.scripting_enabled) {
        Ok(()) => Corpus::Keep,
        Err(message) => panic!("{message}"),
    }
});

#![no_main]

use std::sync::Once;
use std::sync::atomic::{AtomicU64, Ordering};

use htmd_fuzz::generator::generate;
use htmd_fuzz::options::{Target, decode};
use htmd_fuzz::oracle;
use htmd_fuzz::scope::out_of_scope;
use libfuzzer_sys::{Corpus, fuzz_target};

/// Inputs the scope check rejected since the run started.
static REJECTED: AtomicU64 = AtomicU64::new(0);
/// Inputs generated since the run started.
static EXECUTED: AtomicU64 = AtomicU64::new(0);
static REGISTERED: Once = Once::new();

/// Print the count in libFuzzer's `stat::` format. At exit the line follows
/// the statistics of `-print_final_stats=1`.
extern "C" fn print_rejected() {
    eprintln!(
        "stat::round_trip_structured_scope_rejected: {} of {}",
        REJECTED.load(Ordering::Relaxed),
        EXECUTED.load(Ordering::Relaxed)
    );
}

// libFuzzer offers no user statistic and the target no exit hook.
unsafe extern "C" {
    fn atexit(callback: extern "C" fn()) -> i32;
}

fuzz_target!(|data: &[u8]| -> Corpus {
    REGISTERED.call_once(|| unsafe {
        atexit(print_rejected);
    });
    let Some(input) = decode(data, Target::RoundTrip) else {
        return Corpus::Reject;
    };
    let html = generate(input.html_bytes);
    if out_of_scope(&html, input.scripting_enabled).is_some() {
        REJECTED.fetch_add(1, Ordering::Relaxed);
    }
    let executed = EXECUTED.fetch_add(1, Ordering::Relaxed) + 1;
    // Also during the run, so a long run shows the proportion as it goes.
    if executed == 1 || executed % 65536 == 0 {
        print_rejected();
    }
    match oracle::round_trip(&html, input.options, input.scripting_enabled) {
        Ok(()) => Corpus::Keep,
        Err(message) => panic!("{message}"),
    }
});

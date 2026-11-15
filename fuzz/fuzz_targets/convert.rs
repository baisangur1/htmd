#![no_main]

use htmd_fuzz::options::{Target, decode};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = decode(data, Target::Convert);
});

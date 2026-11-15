#![no_main]

use std::thread;

use htmd::HtmlToMarkdown;
use htmd::options::Options;
use htmd_fuzz::options::{Target, decode};
use htmd_fuzz::render;
use htmd_fuzz::text::check_text;
use libfuzzer_sys::{Corpus, fuzz_target};

const STACK_SIZE: usize = 8 << 20;

/// Convert on a new thread with `STACK_SIZE`; see "Targets > convert".
fn convert_on_new_thread(html: &str, options: &Options, scripting_enabled: bool) -> String {
    thread::scope(|scope| {
        thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(scope, || {
                HtmlToMarkdown::builder()
                    .options(options.clone())
                    .scripting_enabled(scripting_enabled)
                    .build()
                    .convert(html)
                    .expect("convert returned Err")
            })
            .unwrap()
            .join()
            .unwrap()
    })
}

fuzz_target!(|data: &[u8]| -> Corpus {
    let Some(input) = decode(data, Target::Convert) else {
        return Corpus::Reject;
    };
    let Ok(html) = str::from_utf8(input.html_bytes) else {
        return Corpus::Reject;
    };
    let first = convert_on_new_thread(html, &input.options, input.scripting_enabled);
    let second = convert_on_new_thread(html, &input.options, input.scripting_enabled);
    assert_eq!(first, second, "second conversion differs");
    check_text(html, &render(&first), input.scripting_enabled).expect("lost text");
    Corpus::Keep
});

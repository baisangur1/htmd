# Review: `round_trip_structured` and seeds/dictionary vs. `fuzzer/fuzzer-spec.md`

Checked against "Targets > round_trip_structured" and "Seeds and dictionary":
`fuzz/src/generator.rs`, `fuzz/fuzz_targets/round_trip_structured.rs`,
`fuzz/htmd.dict`, `fuzz/seeds/convert/`, `fuzz/seeds/round_trip/` (and
`fuzz/make_seeds.py`, which regenerates the seeds byte-identically).

## Findings

1. **Numbered words are not delimited, so the text check cannot reliably name
   the lost word** (`generator.rs`, `build_text`, the `byte % 8 == 6` arm).

   The spec inserts `w0`, `w1`, ... "so that the text check names the word
   lost". `text.rs` concatenates only the alphanumerics of all text nodes and
   `numbered_words` reads `w` followed by every digit after it. The generator
   writes `w{n}` with nothing after it, and its alphabet is full of digits
   (`0`-`9` in `OTHER_CHARS`, and every text node starts with a sequence, two of
   which are `1. A` and `1) A`). Since whitespace and punctuation are dropped
   before the words are read, any digit after a word with only
   non-alphanumerics in between merges into it: a generated sample
   `...-w0<caption ...>---é{!w1|22.<div...` reads as `w0`, `w122`. In 20 000
   random inputs, `w0` is followed *directly* by a digit in 5.7 % of them; the
   merged-across-punctuation rate is higher. A lost `w1` there is reported as
   the nonexistent `w122` (or not at all when another merged word hides it).
   Follow each word with a letter that cannot be a digit (e.g. `w{n}x`) or
   otherwise terminate it.

2. **The scope-rejected count is not printed in the documented run**
   (`round_trip_structured.rs`, `atexit(print_rejected)`).

   The spec requires the target to print the count "with libFuzzer's
   statistics". It is printed only from an `atexit` handler, but libFuzzer
   ends with `_Exit`, which skips `atexit`, on every exit except a bounded run
   (`-runs`, `-max_total_time`): Ctrl-C (`Fuzzer::InterruptCallback`, "don't
   perform any at-exit actions"), a crash/panic (`CrashCallback`), a timeout,
   OOM, and a graceful-exit request (`libfuzzer-sys-0.4.13/libfuzzer/FuzzerLoop.cpp`
   lines 237, 252, 260, 274, 307). The command in "Running",
   `cargo +nightly fuzz run round_trip_structured`, is unbounded and ends with
   Ctrl-C or a finding, so the count never appears there -- exactly when the
   operator needs it to judge whether the run "checks the scope table, not
   htmd". Print it from the target itself (e.g. periodically, every
   power-of-two executions), not only at exit.

3. **Inline `translation_approach.md` rows are seeded without the `<p>`
   `tests/translation_approach/` wraps them in** (`fuzz/seeds/*/t03-*-r01..r02,r05`,
   `t07-*-r01..r09`; `make_seeds.py`, `translation_approach_rows`).

   The spec says the rows are "written out as `tests/translation_approach/`
   does". Those tests convert the inline rows of the code-span and emphasis
   tables inside a paragraph, e.g. `convert_faithful("<p>a<code>x<br><br><br>y</code>b</p>")`
   and `convert_faithful("<p><em><br>a</em></p>")`
   (`tests/translation_approach/inline_faithful.rs` lines 83-96, 164-248),
   while the seeds hold the bare cell, e.g. `a<code>x<br><br><br>y</code>b` and
   `<em><br>a</em>`. The seeds therefore start from a root-level inline run
   rather than the paragraph the table rows describe and the tests check. If
   the bare cell is intended, the spec's list of transformations should say so;
   otherwise wrap these rows in `<p>`.

4. **The `six_children` test's comment and setup disagree**
   (`generator.rs`, `tests::six_children`).

   The comment says "One root element", but the first byte `1u8` is the root
   count byte and gives `1 + 1 % 6 = 2` root nodes; the second only fails to
   materialize because the bytes run out. Use `0u8` for one root node, or fix
   the comment.

## Checked, no finding

* Element list (matches the built-in handlers in `src/element_handler/mod.rs`
  minus `head`/`body`/`html`, plus the math spans and the handler-less tags),
  attribute list and presence/empty/value split, `language-a` alone or after
  another class, the text alphabet (all 27 syntax and 27 other characters are
  reachable from `build_text` despite the `byte % 8` word selector), the six
  words, the 16 sequences at the start of every text node and after LF/CR/CRLF,
  depth 8 and 6 children, the one-in-twelve broken markup, and neighbors on
  half of the inputs (`first % 4` in {0, 2}).
* Option decoding of the target (`Target::RoundTrip`) and the scope count
  itself.
* `fuzz/htmd.dict` holds every item the spec lists.
* Every seed starts with the 13-byte prefix `00..00 01 00` (byte 11 is
  `translation_mode`, default `Pure`, so `1` decodes to `Faithful`); all seeds
  are UTF-8; no `round_trip` seed exceeds 4096 bytes; `hacker-news.html`
  (43 164 bytes) and the five deep-nesting seeds (at most 65 528 bytes) are in
  `convert` only; the Lists table is excluded; `<br><br>...<br>` rows have a
  one- and a three-`<br>` variant; table body-cell rows sit in the one-column
  table; all 142 turndown cases are present.

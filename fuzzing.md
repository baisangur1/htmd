Fuzzing strategy
================

This document specifies how htmd is fuzzed. The fuzzer is an executable check of [translation_approach.md](translation_approach.md): wherever that document says what the Faithful translation of some HTML must be, the fuzzer searches for
HTML which violates it.

Goals and non-goals
-------------------

The fuzzer finds:

1. Panics, stack overflows, and other crashes in `HtmlToMarkdown::convert` under
   any options and either translation mode.
2. Conversions which do not terminate in reasonable time or memory.
3. Faithful translations which do not round-trip: HTML which, translated to CommonMark by htmd and read back to HTML by pulldown-cmark, is not the HTML it started as.
4. Non-determinism: the same converter giving two results for one input.

Layout
------

The fuzzer is a [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) crate in `fuzz/`, which drives libFuzzer and requires a nightly toolchain:

* `fuzz/Cargo.toml` declares its own `[workspace]`, so `cargo build`, `cargo clippy` and `cargo test` in the repository root neither build nor require it, and CI is unchanged.
* `fuzz/src/lib.rs` holds what the targets share: option decoding, the HTML generator, the scope check, and the normalizer.
* `fuzz/fuzz_targets/` holds one file per target.
* `fuzz/seeds/` holds the seed corpus and `fuzz/htmd.dict` the dictionary; both are committed. `fuzz/corpus/` and `fuzz/artifacts/` are not.

Its dependencies are `htmd` (by path), `libfuzzer-sys`, `arbitrary`, and the same `html5ever`, `markup5ever_rcdom` and `pulldown-cmark` versions as htmd's `[dev-dependencies]`, so the fuzzer reads Markdown exactly as the tests do, plus `pulldown-cmark-escape` at the version pulldown-cmark itself locks.

Input decoding
--------------

A fuzz input is a byte string decoded with `arbitrary::Unstructured`. The first bytes choose the `Options`; every field is fuzzed:
* each enum field takes one of its variants;
* `ul_bullet_spacing` and `ol_number_spacing` take any `u8`;
* `preformatted_code` takes either value;
* `translation_mode` takes either value in the `convert` target and is fixed to
  `TranslationMode::Faithful` in the round-trip targets;
* `scripting_enabled` on the builder takes either value.

The remaining bytes are the HTML. Raw targets take them as UTF-8 and reject an input which is not valid UTF-8, since `convert` accepts only a `&str`.

Targets
-------
### `convert`

Converts raw HTML under any options. The target fails if:

1. `convert` panics, overflows the stack, or returns `Err`;
2. a second `convert` of the same input by the same `HtmlToMarkdown` returns a different string. This catches state which outlives one conversion, such as the reference counter of `LinkStyle::Referenced`.

Run with `-max_len=65536 -timeout=2 -rss_limit_mb=2048`. The README reports 16 ms for a 1.37 MB page, so a 64 KiB input taking two seconds is a superlinear path, not a slow machine. Arbitrarily deep nesting is in scope: html5ever does not bound tree depth, and a stack overflow in the DOM walker is a finding.

### `round_trip`

Checks the Faithful translation of raw HTML. Given input `H0`:

1. Reject `H0` if it is out of scope (see "Scope").
2. `M1 = convert(H0)`, Faithful mode.
3. `H1 = render(M1)`, where `render` is pulldown-cmark with
   `ENABLE_TABLES | ENABLE_MATH` — the GFM tables of the "Tables" section and
   the math of the "Math" section — and no other extension.
4. Fail unless `N(H0) == N(H1)`, where `N` is the normalizer below.
5. `H2 = render(convert(H1))`. Fail unless `N(H1) == N(H2)`.

Step 5 repeats the trip from the canonical HTML pulldown-cmark writes. It is
redundant where step 4 is exact, but it still catches a translation which
drifts on every trip in cases where `N` is too coarse to see the first drift.

Run with `-max_len=4096`: the normalizer and the tree comparison are linear, but
short inputs minimize to readable test cases.

### `round_trip_structured`

The `round_trip` oracle on HTML built by the generator.

The generator builds a tree from `Unstructured`, then serializes it:

* Elements: every tag with a built-in handler (`a`, `b`, `blockquote`, `br`,
  `caption`, `code`, `em`, `h1`-`h6`, `hr`, `i`, `img`, `li`, `ol`, `p`, `pre`,
  `strong`, `table`, `tbody`, `td`, `th`, `thead`, `tr`, `ul`), the math spans
  `<span class="math math-inline">` and `<span class="math math-display">`, and
  elements without a handler (`span`, `div`, `del`, `sub`, `sup`, `kbd`).
* Attributes: `href`, `src`, `alt`, `title`, `class`, `id`, `start`, `align`,
  each present or absent, empty or not.
* Comments.
* Text drawn mostly from characters which are CommonMark syntax somewhere —
  `` \ ` * _ [ ] ( ) < > & ! | $ # ~ - + = . : ' " % { } `` — plus digits, `a`,
  space, tab, LF, CR, U+00A0, and a Unicode punctuation character which is not
  ASCII (`¡`), since flanking depends on Unicode punctuation and whitespace.
* At most 8 levels deep and 6 children per element.

The generator ignores HTML content models on purpose: `<h1><p>a</p></h1>` and
`<em><table>` are the inputs the "Translating HTML nodes" section is about.
Since html5ever repairs such trees, `N(H0)` is computed from the parsed input,
never from the generator's tree.

Comparison
----------

Both sides are parsed by html5ever as full documents, with the same
`scripting_enabled` as the conversion, and the children of `<body>` are
compared as trees: element names, attribute sets (order ignored), text, and
comment data. Character references are decoded by the parser, so `&#10;` in
`M1` and a line feed in `H0` compare equal, as the "Translating HTML nodes"
section intends.

### Normalizer

`N` erases the differences which translation_approach.md accepts and nothing
else. Each rule cites what accepts the difference. A difference with no
citation is either an htmd bug or a gap in translation_approach.md; it is never
fixed by adding a rule here alone.

1. **Whitespace.** Outside `<pre>` and `<textarea>`, a run of ASCII whitespace
   becomes one space; whitespace at the start and end of a block element's
   content, and whitespace-only text between block siblings, is removed. HTML
   renders such whitespace identically, and htmd collapses it by design.
2. **Implied paragraphs.** In `<body>`, `<blockquote>` and `<li>`, a maximal run
   of inline children is wrapped in a `<p>`, unless it already is one. The
   tables of "At the document root" and "Blockquotes" call the inserted `<p>`
   lossy but accepted.
3. **Emphasis names.** `<i>` becomes `<em>` and `<b>` becomes `<strong>`. The
   "Translating HTML nodes" section treats each pair as one CommonMark
   construct, and `tests/translation_approach_tests.rs` expects `<i>a</i>` as
   `*a*`.
4. **Code blocks.** A `<pre>` whose content is not a single `<code>` gets its
   content wrapped in one. On the output side only, one trailing LF is removed from that `<code>`: a fenced or indented code block always ends its content  with a line ending.
5. **URLs.** Input `href` and `src` values are passed through
   `pulldown_cmark_escape::escape_href`, the percent-encoding pulldown-cmark's
   HTML writer applies to every destination.
6. **Comments.** A run of whitespace in comment data becomes one space ("For
   simplicity, whitespace including newlines is collapsed, a lossy
   translation"). This also erases the difference for comments in block context,
   where it is not lossy; the fuzzer accepts that imprecision.
7. **Math.** In a math span the text is compared after rule 1 and, for inline
   math, after trimming; this is steps 2-3 of the "Math" section.

### Scope

An input is out of scope, and rejected before step 2 of `round_trip`, when its
parsed DOM holds any of the following. Each is a case translation_approach.md
calls lossy or unspecified; an entry is deleted once that document specifies
the case.

| Parsed DOM holds                                                            | Reason (translation_approach.md)                    |
| --------------------------------------------------------------------------- | --------------------------------------------------- |
| `<ul>`, `<ol>` or `<li>`                                                    | "Lists": "under development. Ignore it."            |
| `<script>`, `<style>` or `<textarea>`                                       | Type 1 content: mis-translated inline, unspecified  |
| `<pre>` in an inline context whose text holds a line ending                 | Type 1 content: "newlines … are lost"               |
| `<iframe>`, `<xmp>`, `<noscript>`, `<noembed>`, `<noframes>`, `<plaintext>` | Special case: unspecified                           |
| A doctype, or any child of `<head>`                                         | Special case: doctype dropped; head not specified   |
| A math span whose text holds `%`                                            | "Math": comments are replaced, changing source text |

`HTMD_FUZZ_LISTS=1` lifts the first row, so that work on the "Lists" section can
use the fuzzer before that section is final.

Seeds and dictionary
--------------------

The seed corpus is:

* the *HTML in* column of every table in translation_approach.md, one file per
  row, with `<br><br>...<br>` expanded to one and to three `<br>`s as
  `tests/translation_approach_tests.rs` does;
* the cases of `tests/html/turndown_test_index.html`;
* the pages in `examples/page-to-markdown/html/`, for the `convert` target only,
  since they exceed `-max_len` of the others.

`fuzz/htmd.dict` lists every tag name the generator emits, the attributes
above, `class="math math-inline"`, `class="math math-display"`, `<!--`, `-->`,
`<![CDATA[`, `&#10;`, `&#13;`, `&amp;`, `&lt;`, and the CommonMark syntax
characters of the generator's text alphabet.

Triage
------

A failing input is minimized with `cargo fuzz tmin` and classified:

1. **htmd bug.** The minimized input contradicts a row or rule of
   translation_approach.md. Fix htmd and add the input as a test in the
   `tests/` file matching the feature, through `round_trip` or
   `convert_faithful` in `tests/common/mod.rs`.
2. **Specification gap.** translation_approach.md does not say what the input
   should become. Add a row to the matching table there first; the test then
   follows as in case 1.
3. **Normalizer bug.** `N` hides or invents a difference against the rules
   above. Fix `fuzz/src/lib.rs` and add the input to `fuzz/seeds/`.
4. **Reference bug.** html5ever or pulldown-cmark disagrees with the HTML
   standard or the CommonMark spec. Report it upstream and add a scope entry
   citing the issue.

A crash or hang from the `convert` target is always case 1.

Running
-------

```
cargo +nightly fuzz run convert -- -max_len=65536 -timeout=2 -rss_limit_mb=2048
cargo +nightly fuzz run round_trip -- -max_len=4096 -dict=fuzz/htmd.dict
cargo +nightly fuzz run round_trip_structured
```

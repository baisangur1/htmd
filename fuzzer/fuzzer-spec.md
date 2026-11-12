Fuzzing strategy
================

This document specifies a fuzzing strategy for htmd. When implemented, the fuzzer will be an executable check of [translation_approach.md](../translation_approach.md): wherever that document says what the Faithful translation of some HTML must be, the fuzzer searches for HTML which violates it. Wherever that document calls a case lossy or unspecified, the fuzzer must not report it.

Goals and non-goals
-------------------

When implemented, the fuzzer is intended to find:

1. Panics, stack overflows, and other crashes in `HtmlToMarkdown::convert` under any options and either translation mode.
2. Conversions which do not terminate in reasonable time or memory.
3. Faithful translations which do not round-trip: HTML which, translated to CommonMark by htmd and read back to HTML by pulldown-cmark, is not the HTML it started as.
4. Non-determinism: the same input and options giving two results.

Out of scope:

* Pure mode correctness. Pure mode drops what CommonMark cannot spell, so no round-trip oracle exists for it; it is fuzzed for crashes only.
* Custom handlers registered through `add_handler` and `skip_tags`. A handler may write output this design does not describe (see "Translating HTML nodes"),
  so a mismatch there says nothing about htmd.
* The `ElementHandler` event API.
* Correctness of html5ever and pulldown-cmark. Both are treated as reference implementations; a bug in either is reported upstream (see "Triage").

Layout
------

The fuzzer is a [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) crate in `fuzz/`, which drives libFuzzer and requires a nightly toolchain:

* `fuzz/Cargo.toml` declares its own `[workspace]`, so `cargo build`, `cargo clippy` and `cargo test` in the repository root neither build nor require it, and CI is unchanged.
* `fuzz/src/lib.rs` holds what the targets share: option decoding, the HTML generator, the scope check, and the normalizer.
* `fuzz/fuzz_targets/` holds one file per target.
* `fuzz/seeds/` holds the seed corpus and `fuzz/htmd.dict` the dictionary.

Input decoding
--------------

A fuzz input is a byte string. Its first 13 bytes are intended to choose the options, one byte per field, in this order: the twelve fields of `Options` in
declaration order, then `scripting_enabled` on the builder. An input shorter than 13 bytes is rejected. Byte 0 always decodes to the default, so a prefix of
zero bytes gives
`Options::default()` with scripting enabled:

* an enum field takes the variant `(d + b) mod n` in declaration order, where `d` is the index of its default, `b` the byte and `n` the number of variants;
* `ul_bullet_spacing` and `ol_number_spacing` take the default plus the byte, wrapping, so values outside 1 through 4 exercise the clamping `options.rs` documents;
* `preformatted_code` and `scripting_enabled` take the default, flipped when the byte is odd.

In the round-trip targets three fields ignore their byte:

* `translation_mode` is `TranslationMode::Faithful`;
* `preformatted_code` is `true`. With `false`, inline code has its whitespace trimmed and collapsed by design (`options.rs`), which no round trip survives;
* `link_reference_style` is `LinkReferenceStyle::Full`. `Collapsed` and `Shortcut` use the link text as the label, so two links with the same text
  and different destinations read back with the first one's, and translation_approach.md marks both styles for removal ("Links").

The remaining bytes are the HTML. Raw targets take them as UTF-8 and reject an input which is not valid UTF-8, since `convert` accepts only a `&str`. Structured targets pass them to `arbitrary::Unstructured` and build the HTML with the generator below.

Targets
-------

### `convert`

Converts raw HTML under any options. The target fails if:w

1. `convert` panics, overflows the stack, or returns `Err`;
2. a second conversion of the same input, by a freshly built `HtmlToMarkdown`
   on a new thread, returns a different string. The `LinkStyle::Referenced`
   definitions are kept in a thread-local stack (`anchor.rs`), not in the
   converter, so only a new thread starts from clean state. A leak from an
   earlier input is reported on the input which sees it; when `tmin` cannot
   reproduce such a failure alone, it is reproduced by replaying the inputs of
   that run in order.

Run with `-max_len=65536 -timeout=2 -rss_limit_mb=2048`. The README reports
16 ms for a 1.37 MB page, so a 64 KiB input taking two seconds is a superlinear
path. Arbitrarily deep nesting is in scope: html5ever does
not bound tree depth, and a stack overflow in the DOM walker is a finding.

### `round_trip`

Checks the Faithful translation of raw HTML. Given input `H0`:

1. Reject `H0` if it is out of scope (see "Scope").
2. `M1 = convert(H0)`, Faithful mode.
3. `H1 = render(M1)`, where `render` is pulldown-cmark with
   `ENABLE_TABLES | ENABLE_MATH` — the GFM tables of the "Tables" section and
   the math of the "Math" section — and no other extension.
4. Fail unless `H1` matches `H0` (see "Comparison").
5. `H2 = render(convert(H1))`. Fail unless `H2` matches `H1`, with `H1` as the
   input side.

Step 5 repeats the trip from the canonical HTML pulldown-cmark writes. It is
redundant where step 4 is exact, but it still catches a translation which
drifts on every trip in cases where the comparison is too coarse to see the
first drift.

Run with `-max_len=4096`: the normalizer and the tree comparison are linear, but
short inputs minimize to readable test cases.

### `round_trip_structured`

The `round_trip` oracle on HTML built by the generator. Byte-level mutation
rarely produces well-formed nested markup with meaningful text, which is where
the rules of translation_approach.md live; the generator produces it directly.

The generator builds a tree from `Unstructured`, then serializes it:

* Elements: every tag with a built-in handler (`a`, `b`, `blockquote`, `br`,
  `caption`, `code`, `em`, `h1`-`h6`, `hr`, `i`, `img`, `p`, `pre`, `strong`,
  `table`, `tbody`, `td`, `tfoot`, `th`, `thead`, `tr`), the math spans
  `<span class="math math-inline">` and `<span class="math math-display">`, and
  elements without a handler (`span`, `div`, `del`, `sub`, `sup`, `kbd`,
  `colgroup`, `col`). `li`, `ol` and `ul` are emitted only under
  `HTMD_FUZZ_LISTS=1`; otherwise the scope would reject nearly every tree.
* Attributes: `href`, `src`, `alt`, `title`, `class`, `id`, `start`, `align`,
  each present or absent, empty or not. A `class` value is drawn from the text
  below or is `language-a`, alone or after another class, which a code block
  writes as its info string.
* Comments.
* Text drawn mostly from characters which are CommonMark syntax somewhere —
  `` \ ` * _ [ ] ( ) < > & ; ! | $ # ~ - + = . : ' " % { } @ `` — plus digits,
  `a`, space, tab, LF, CR, U+00A0, and a Unicode punctuation character which is
  not ASCII (`¡`), since flanking depends on Unicode punctuation and
  whitespace. Among them it inserts the words `lt`, `amp`, `copy`, `div`, `pre`
  and `CDATA`, so the text can spell character references and the starts of
  HTML blocks of types 1-6.
* At most 8 levels deep and 6 children per element.

The generator ignores HTML content models on purpose: `<h1><p>a</p></h1>` and
`<em><table>` are the inputs the "Translating HTML nodes" section is about.
Since html5ever repairs such trees, `H0` is the parsed input, never the
generator's tree.

Comparison
----------

Both sides are parsed by html5ever as full documents, with the same
`scripting_enabled` as the conversion, and compared as trees from the document
node down: element names, attribute sets (order ignored), text, and comment
data. A comment before `<html>`, after `</body>` or after `</html>` is outside
`<body>` but is still written by htmd, so it is compared too. Character
references are decoded by the parser, so `&#10;` in `M1` and a line feed in
`H0` compare equal, as the "Translating HTML nodes" section intends.

Each side first goes through the normalizer `N`. Then two rules match the input
side against the output side, since the difference they accept runs one way
only:

1. **Implied paragraphs.** In `<body>` and `<blockquote>` (and in `<li>` under
   `HTMD_FUZZ_LISTS=1`), a maximal run of inline children on the input side
   matches the same run on the output side either as it is or wrapped in one
   `<p>`. An inline child is a text node, a comment, or an element whose tag
   name is in neither the type 1 nor the type 6 list of CommonMark HTML blocks.
   A `<p>` on the input side matches only a `<p>`, so the paragraphs row which
   keeps `<p><br></p>` still fails when the `<p>` is lost. The tables of "At
   the document root" and "Blockquotes" call the inserted `<p>` lossy but
   accepted.
2. **URLs.** The `href` of an `<a>` and the `src` of an `<img>` on the input
   side match the output value as it is or after
   `pulldown_cmark_escape::escape_href`, the percent-encoding pulldown-cmark's
   HTML writer applies to every destination it writes. The `&amp;` and `&#x27;`
   which `escape_href` emits are taken as `&` and `'`, since attribute values
   are compared decoded. The value as it is matches an element htmd wrote as
   HTML, which pulldown-cmark passes through untouched.

### Normalizer

`N` erases the differences which translation_approach.md accepts and nothing
else, and is applied alike to both sides. Each rule cites what accepts the
difference. A difference with no citation is either an htmd bug or a gap in
translation_approach.md; it is never fixed by adding a rule here alone.

1. **Whitespace.** Outside `<pre>` and `<textarea>`, a run of ASCII whitespace
   becomes one space; whitespace at the start and end of a block element's
   content, and whitespace-only text between block siblings, is removed. HTML
   renders such whitespace identically, and htmd collapses it by design ("All
   raw HTML inlines will have whitespace collapsed in their contents").
2. **Emphasis names.** `<i>` becomes `<em>` and `<b>` becomes `<strong>`.
   `tests/translation_approach_tests.rs` expects `<i>a</i>` as `*a*`, but
   translation_approach.md states the merge only for Pure mode ("Emphasis and
   strong emphasis"). This rule therefore lands together with a row for `<i>`
   and `<b>` in that section's table.
3. **Emphasis whitespace.** Unicode whitespace at the start and end of an
   `<em>` or `<strong>` moves out of it, before and after the element. Step 3
   of the emphasis approach does this, and rule 1 cannot see it: the
   whitespace changes element, and U+00A0 is not ASCII whitespace.
4. **Code blocks.** A `<pre>` whose only child is a `<code>` loses one trailing
   LF from that `<code>`. pulldown-cmark ends every code block with a line
   ending, and htmd drops one trailing LF from the content it writes
   (`code.rs`), so `<pre><code>a</code></pre>` and
   `<pre><code>a⏎</code></pre>` translate alike. Any other `<pre>` goes out
   verbatim as HTML and is left unchanged. This rule lands together with a row
   for the trailing LF in the "Code blocks and code spans" table.
5. **Table sections.** In a `<table>` with no `<thead>`, a first row of `<th>`
   cells at the start of the first `<tbody>` moves into a new `<thead>` before
   it, and an empty `<tbody>` right after a `<thead>` is removed. htmd writes
   such a row as the GFM header row, which pulldown-cmark reads back inside a
   `<thead>`; the "Tables" section asks only for "a header row of header cells,
   then one body".
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

| Parsed DOM holds | Reason (translation_approach.md) |
| ---------------- | -------------------------------- |
| `<ul>`, `<ol>` or `<li>` | "Lists": "under development. Ignore it." |
| `<script>` or `<style>` in an inline context | Type 1 content: mis-translated inline |
| `<pre>` or `<textarea>` in an inline context whose text holds a line ending | Type 1 content: "newlines … are lost" |
| A comment, `<script>` or `<style>` holding a blank line inside an element whose tag name is in the type 6 list | Special case: unspecified |
| Inside a `<pre>`, a comment, `<script>` or `<style>` holding `</pre>`, `</script>`, `</style>` or `</textarea>` followed by a line ending | Special case: unspecified |
| `<iframe>`, `<noembed>` or `<noframes>` whose text holds anything but ASCII letters, digits and spaces; `<xmp>` or `<plaintext>`; `<noscript>` when scripting is enabled | Special case: unspecified (the paragraphs table specifies a plain `<iframe>`) |
| `<template>` | Not specified: its content is not among its children |
| A CR (U+000D) in text inside `<pre>` | Not specified: a code block cannot hold a CR, and no reference is decoded there |
| A doctype, or any child of `<head>` | Special case: doctype dropped; head not specified |
| A math span whose text holds `%` | "Math": comments are replaced, changing source text |

`HTMD_FUZZ_LISTS=1` lifts the first row, so that work on the "Lists" section can
use the fuzzer before that section is final.

Seeds and dictionary
--------------------

Every seed file starts with the 13-byte option prefix, all zero bytes except a
1 for `translation_mode`, so that the `convert` target too starts from Faithful
mode with the default options. The HTML which follows comes from:

* the *HTML in* column of every table in translation_approach.md, one file per
  row, written out as `tests/translation_approach_tests.rs` does: `␣` and `⏎`
  replaced by a space and a line feed, `<br><br>...<br>` expanded to one and to
  three `<br>`s, and a body-cell row placed in the one-column table the
  "Tables" section describes;
* the cases of `tests/html/turndown_test_index.html`;
* `examples/page-to-markdown/html/Hacker News.html`, for the `convert` target
  only, since it exceeds `-max_len` of the others. The Wikipedia page there
  exceeds even the 64 KiB of `convert`, which libFuzzer would truncate, so it is
  not a seed.

`fuzz/htmd.dict` lists every tag name the generator emits, the attributes
above, `class="math math-inline"`, `class="math math-display"`,
`class="language-`, `<!--`, `-->`, `<![CDATA[`, `&#10;`, `&#13;`, `&amp;`,
`&lt;`, `&copy;`, and the CommonMark syntax characters of the generator's text
alphabet.

Triage
------

A failing input is minimized with `cargo fuzz tmin` and classified:

1. **htmd bug.** The minimized input contradicts a row or rule of
   translation_approach.md. Fix htmd and add the input as a test in the
   `tests/` file matching the feature: convert it with `convert_faithful_options`
   and the options the input decoded to, and read the result back with
   `render_markdown` and `ENABLE_TABLES | ENABLE_MATH` from
   `tests/common/mod.rs`. The `round_trip` helper there renders with no
   extension and the default options, so it reproduces only inputs which need
   neither.
2. **Specification gap.** translation_approach.md does not say what the input
   should become. Add a row to the matching table there first; the test then
   follows as in case 1.
3. **Normalizer bug.** `N` or a matching rule hides or invents a difference
   against the rules above. Fix `fuzz/src/lib.rs` and add the input to
   `fuzz/seeds/`.
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

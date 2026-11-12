Fuzzing strategy
================

This document specifies the fuzzing strategy for htmd. The fuzzer checks that the Faithful translation round-trips: HTML translated to CommonMark by htmd and read back to HTML must be the HTML it started as. [`translation_approach.md`](../translation_approach.md) supplies the exceptions: wherever that document calls a case lossy or unspecified, the fuzzer must not report it.

Goals and non-goals
-------------------

The fuzzer finds:

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

A fuzz input is a byte string. Its first 13 bytes choose the options, one byte per field, in this order: the twelve fields of `Options` in
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
  and different destinations read back with the first one's, and `translation_approach.md` marks both styles for removal ("Links").

The remaining bytes are the HTML. Raw targets take them as UTF-8 and reject an input which is not valid UTF-8, since `convert` accepts only a `&str`. Structured targets use them as the source of the generator's choices and build the HTML with the generator below.

Targets
-------

### `convert`

Converts raw HTML under any options. The target fails if:

1. `convert` panics, overflows the stack, or returns `Err`;
2. a second conversion of the same input, by a freshly built `HtmlToMarkdown`
   on a new thread, returns a different string. The `LinkStyle::Referenced`
   definitions are kept in a thread-local stack (`anchor.rs`), not in the
   converter, so only a new thread starts from clean state. A leak from an
   earlier input is reported on the input which sees it; when `tmin` cannot
   reproduce such a failure alone, it is reproduced by replaying the inputs of
   that run in order.

Run with `-max_len=65536 -timeout=2 -rss_limit_mb=2048`. `README.md` reports
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
5. `H2 = render(convert(H1))`. Fail unless `H2` matches `H1` under the same
   comparison as step 4 — the tree comparison of "Comparison", not string
   equality — with `H1` as the input side.

Step 5 repeats the trip from the canonical HTML pulldown-cmark writes. It is
redundant where step 4 is exact, but it still catches a translation which
drifts on every trip in cases where the comparison is too coarse to see the
first drift.

Run with `-max_len=4096`: the normalizer and the tree comparison are linear, but
short inputs minimize to readable test cases.

### `round_trip_structured`

The `round_trip` oracle on HTML built by the generator. Byte-level mutation
rarely produces well-formed nested markup with meaningful text, which is where
the rules of `translation_approach.md` live; the generator produces it directly.

The generator builds a tree from the input bytes, then serializes it:

* Elements: every tag with a built-in handler (`a`, `b`, `blockquote`, `br`,
  `caption`, `code`, `em`, `h1`-`h6`, `hr`, `i`, `img`, `li`, `ol`, `p`, `pre`,
  `strong`, `table`, `tbody`, `td`, `tfoot`, `th`, `thead`, `tr`, `ul`), the
  math spans `<span class="math math-inline">` and
  `<span class="math math-display">`, and elements without a handler (`span`,
  `div`, `del`, `sub`, `sup`, `kbd`, `colgroup`, `col`).
* Attributes: `href`, `src`, `alt`, `title`, `class`, `id`, `start`, `align`,
  each present or absent, empty or not. A `class` value is drawn from the text
  below or is `language-a`, alone or after another class, which a code block
  writes as its info string.
* Comments.
* Text drawn mostly from characters which are CommonMark syntax somewhere —
  `` \ ` * _ [ ] ( ) < > & ; ! | $ # ~ - + = . : ' " % { } @ `` — plus digits,
  `a`, `A`, space, tab, LF, CR, form feed, the Unicode whitespace characters
  U+00A0, U+2003 and U+3000, and a Unicode punctuation character which is not
  ASCII (`¡`), since flanking depends on Unicode punctuation and whitespace.
  Among them it inserts the words `lt`, `amp`, `copy`, `div`, `pre` and
  `CDATA`, so the text can spell character references and the starts of HTML
  blocks of types 1-6.
* Sequences which are CommonMark block syntax at the start of a line, placed at
  the start of a text node or after a line ending in it: `1. A`, `1) A`, `* a`,
  `- a`, `+ a`, `---`, `***`, `___`, `===`, `# a`, `> a`, `` ``` ``, `~~~`,
  four spaces, `[a]: b` and `| a |`.
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

1. **Implied paragraphs.** In `<body>`, `<blockquote>` and `<li>`, a maximal
   run of inline children on the input side
   matches the same run on the output side either as it is or wrapped in one
   `<p>`. An inline child is a text node, a comment, or an element whose tag
   name is in neither the type 1 nor the type 6 list of CommonMark HTML blocks.
   A `<p>` on the input side matches only a `<p>`, so the paragraphs row which
   keeps `<p><br></p>` still fails when the `<p>` is lost. The tables of "At
   the document root" and "Blockquotes" call the inserted `<p>` lossy but
   accepted.
2. **URLs.** The `href` of an `<a>` and the `src` of an `<img>` on the input
   side match the output value as it is or after the percent-encoding
   pulldown-cmark's HTML writer applies to every destination it writes. The
   `&amp;` and `&#x27;` which that encoding emits are taken as `&` and `'`,
   since attribute values are compared decoded. The value as it is matches an element htmd wrote as
   HTML, which pulldown-cmark passes through untouched.

### Normalizer

`N` erases the differences which `translation_approach.md` accepts and nothing
else, and is applied alike to both sides. Each rule cites what accepts the
difference. A difference with no citation is reported; it is never fixed by
adding a rule here alone. In particular `N` keeps `<i>` apart from `<em>`,
`<b>` apart from `<strong>`, and whitespace at the start or end of an emphasis
inside it: `translation_approach.md` merges the names and moves the whitespace
out for Pure mode only ("Emphasis and strong emphasis").

1. **Whitespace.** Outside `<pre>` and `<textarea>`, a run of ASCII whitespace
   becomes one space; whitespace at the start and end of a block element's
   content, and whitespace-only text between block siblings, is removed. HTML
   renders such whitespace identically, and htmd collapses it by design ("All
   raw HTML inlines will have whitespace collapsed in their contents").
2. **Code blocks.** A `<pre>` whose only child is a `<code>` loses one trailing
   LF from that `<code>`. pulldown-cmark ends every code block with a line
   ending, and htmd drops one trailing LF from the content it writes
   (`code.rs`), so `<pre><code>a</code></pre>` and
   `<pre><code>a⏎</code></pre>` translate alike. Any other `<pre>` goes out
   verbatim as HTML and is left unchanged. This rule lands together with a row
   for the trailing LF in the "Code blocks and code spans" table.
3. **Table sections.** In a `<table>` with no `<thead>`, a first row of `<th>`
   cells at the start of the first `<tbody>` moves into a new `<thead>` before
   it, and an empty `<tbody>` right after a `<thead>` is removed. htmd writes
   such a row as the GFM header row, which pulldown-cmark reads back inside a
   `<thead>`; the "Tables" section asks only for "a header row of header cells,
   then one body".
4. **List start.** An `<ol>` loses a `start` attribute whose value is `1`. An
   ordered list starts at 1 by default in HTML, and pulldown-cmark writes
   `start` only for another number.
5. **Comments.** A run of whitespace in comment data becomes one space ("For
   simplicity, whitespace including newlines is collapsed, a lossy
   translation"). This also erases the difference for comments in block context,
   where it is not lossy; the fuzzer accepts that imprecision.
6. **Math.** In a math span the text is compared after rule 1 and, for inline
   math, after trimming; this is steps 2-3 of the "Math" section.

### Scope

An input is out of scope, and rejected before step 2 of `round_trip`, when its
parsed DOM holds any of the following. Each is a case `translation_approach.md`
calls lossy or unspecified; an entry is deleted once that document specifies
the case.

| Parsed DOM holds | Reason (`translation_approach.md`) |
| ---------------- | -------------------------------- |
| `<script>` or `<style>` in an inline context | Type 1 content: mis-translated inline |
| `<pre>` or `<textarea>` in an inline context whose text holds a line ending | Type 1 content: "newlines … are lost" |
| A comment, `<script>` or `<style>` holding a blank line inside an element whose tag name is in the type 6 list | Special case: unspecified |
| Inside a `<pre>`, a comment, `<script>` or `<style>` holding `</pre>`, `</script>`, `</style>` or `</textarea>` followed by a line ending | Special case: unspecified |
| `<iframe>`, `<noembed>` or `<noframes>` whose text holds anything but ASCII letters, digits and spaces; `<xmp>` or `<plaintext>`; `<noscript>` when scripting is enabled | Special case: unspecified (the paragraphs table specifies a plain `<iframe>`) |
| `<template>` | Not specified: its content is not among its children |
| A CR (U+000D) in text inside `<pre>` | Not specified: a code block cannot hold a CR, and no reference is decoded there |
| A doctype, or any child of `<head>` | Special case: doctype dropped; head not specified |
| A math span whose text holds `%` | "Math": comments are replaced, changing source text |

Lists are in scope. The "Lists" section of `translation_approach.md` is under
development, so the round trip checks the list translation htmd implements,
with no list-specific exclusion.

Seeds and dictionary
--------------------

Every seed file starts with the 13-byte option prefix, all zero bytes except a
1 for `translation_mode`, so that the `convert` target too starts from Faithful
mode with the default options. The HTML which follows comes from:

* the *HTML in* column of every table in `translation_approach.md`, one file per
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
`&lt;`, `&copy;`, `&nbsp;`, `&emsp;`, and the CommonMark syntax characters and
start-of-line sequences of the generator's text.

Triage
------

A failing input is minimized with `cargo fuzz tmin` and classified by where
the difference arises:

1. **Normalizer bug.** `N` or a matching rule hides or invents a difference
   against the rules above. Fix `fuzz/src/lib.rs` and add the input to
   `fuzz/seeds/`.
2. **Reference bug.** html5ever or pulldown-cmark disagrees with the HTML
   standard or the CommonMark spec. Report it upstream and add a scope entry
   citing the issue.
3. **htmd finding.** Anything else: the round trip lost or changed something
   the comparison does not accept. Report it with the minimized input, the
   options it decoded to, `M1`, and the subtrees which differ. A finding which
   is fixed in htmd becomes a test in the `tests/` file matching the feature:
   convert the input with `convert_faithful_options` and the decoded options,
   and read the result back with `render_markdown` and
   `ENABLE_TABLES | ENABLE_MATH` from `tests/common/mod.rs`. The `round_trip`
   helper there renders with no extension and the default options, so it
   reproduces only inputs which need neither. A finding which is accepted as
   lossy becomes a scope entry or a normalizer rule citing the text of
   `translation_approach.md` which accepts it.

A crash or hang from the `convert` target is always case 3.

Running
-------

```
cargo +nightly fuzz run convert -- -max_len=65536 -timeout=2 -rss_limit_mb=2048
cargo +nightly fuzz run round_trip -- -max_len=4096 -dict=fuzz/htmd.dict
cargo +nightly fuzz run round_trip_structured
```

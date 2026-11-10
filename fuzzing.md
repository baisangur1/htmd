Fuzzing strategy
================

This document specifies a proposed fuzzing strategy for htmd. When implemented,
the fuzzer will be an executable check of
[translation_approach.md](translation_approach.md): wherever that document says
what the Faithful translation of some HTML must be, the fuzzer searches for
HTML which violates it. Wherever that document calls a case lossy or
unspecified, the fuzzer must not report it.

**Implementation status:** this branch has no `fuzz/` directory. A minimal
cargo-fuzz crate (manifest, lockfile, and an empty `fuzz_target_1`) exists only
in the round 5 reviewer's local checkout and is not committed. The targets,
generator, normalizer, corpus, dictionary, and fuzz manifest described below
are requirements for a future implementation, not repository artifacts. Root manifest and source claims were checked against the current
checkout; details that depend on the missing fuzz implementation remain
unverified.

Review history
--------------

| Round | Author | Model | Findings | Main changes made |
| ----- | ------ | ----- | -------- | ----------------- |
| 1 | Baisangur Yunussov | — | — | Initial specification |
| 2 | Baisangur Yunussov | Claude Opus 5.5 | 18 | Seeds-friendly option prefix; lists out of the generator; emphasis whitespace, code-block LF and table header normalized; one-sided matches for implied `<p>` and URLs; scope narrowed for iframe/noscript/textarea/script, nested blocks, template and CR added; pinned deps |
| 3 | Mariya Belyavskaya | GPT-6 Luna | 2 | Marked the strategy as proposed and separated verified root dependency versions from unverified fuzz-crate claims |
| 4 | Mariya Belyavskaya | GPT-6 Luna | 0 | No actionable technical defects remained after checking the revised strategy against the available source and translation specification |
| 5 | Mariya Belyavskaya | GPT-5.6 Luna | 2 | Implementation status and dependency paragraph described a minimal fuzz crate; reverted after review because that crate is not committed |
| 6 | Arsen Alimzhan | Codex (GPT-6) | 2 | Seed initialization and timeout interpretation; proposed, not yet applied |

Goals and non-goals
-------------------

When implemented, the fuzzer is intended to find:

1. Panics, stack overflows, and other crashes in `HtmlToMarkdown::convert` under
   any options and either translation mode.
2. Conversions which do not terminate in reasonable time or memory.
3. Faithful translations which do not round-trip: HTML which, translated to
   CommonMark by htmd and read back to HTML by pulldown-cmark, is not the HTML
   it started as.
4. Non-determinism: the same input and options giving two results.

Out of scope:

* Pure mode correctness. Pure mode drops what CommonMark cannot spell, so no
  round-trip oracle exists for it; it is fuzzed for crashes only.
* Custom handlers registered through `add_handler` and `skip_tags`. A handler
  may write output this design does not describe (see "Translating HTML nodes"),
  so a mismatch there says nothing about htmd.
* The `ElementHandler` event API.
* Correctness of html5ever and pulldown-cmark. Both are treated as reference
  implementations; a bug in either is reported upstream (see "Triage").

Layout
------

The proposed fuzzer is a [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html)
crate in `fuzz/`, which drives libFuzzer and requires a nightly toolchain:

* `fuzz/Cargo.toml` declares its own `[workspace]`, so `cargo build`,
  `cargo clippy` and `cargo test` in the repository root neither build nor
  require it, and CI is unchanged.
* `fuzz/src/lib.rs` holds what the targets share: option decoding, the HTML
  generator, the scope check, and the normalizer.
* `fuzz/fuzz_targets/` holds one file per target.
* `fuzz/seeds/` holds the seed corpus and `fuzz/htmd.dict` the dictionary; both
  are committed. `fuzz/corpus/` and `fuzz/artifacts/` are not.

The proposed dependencies are `htmd` (by path), `libfuzzer-sys`, `arbitrary`,
and `html5ever`, `markup5ever_rcdom`, `pulldown-cmark` and
`pulldown-cmark-escape`. The root `Cargo.lock` resolves the latter four to
0.39.0, 0.39.0+unofficial, 0.13.4 and 0.11.0; pinning those versions in the
future `fuzz/Cargo.toml` would keep the oracle aligned with this checkout. The
fuzz manifest and its lockfile are not committed, so their dependency
declarations and resolution cannot yet be verified.

Input decoding
--------------

A fuzz input is a byte string. Its first 13 bytes are intended to choose the
options, one byte per field, in this order: the twelve fields of `Options` in
declaration order, then `scripting_enabled` on the builder. An input shorter
than 13 bytes is rejected. Byte 0 always decodes to the default, so a prefix of
zero bytes gives
`Options::default()` with scripting enabled:

* an enum field takes the variant `(d + b) mod n` in declaration order, where
  `d` is the index of its default, `b` the byte and `n` the number of variants;
* `ul_bullet_spacing` and `ol_number_spacing` take the default plus the byte,
  wrapping, so values outside 1 through 4 exercise the clamping `options.rs`
  documents;
* `preformatted_code` and `scripting_enabled` take the default, flipped when
  the byte is odd.

In the round-trip targets three fields ignore their byte:

* `translation_mode` is `TranslationMode::Faithful`;
* `preformatted_code` is `true`. With `false`, inline code has its whitespace
  trimmed and collapsed by design (`options.rs`), which no round trip survives;
* `link_reference_style` is `LinkReferenceStyle::Full`. `Collapsed` and
  `Shortcut` use the link text as the label, so two links with the same text
  and different destinations read back with the first one's, and
  translation_approach.md marks both styles for removal ("Links").

The remaining bytes are the HTML. Raw targets take them as UTF-8 and reject an
input which is not valid UTF-8, since `convert` accepts only a `&str`.
Structured targets pass them to `arbitrary::Unstructured` and build the HTML
with the generator below.

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

Run with `-max_len=65536 -timeout=2 -rss_limit_mb=2048`. The README reports
16 ms for a 1.37 MB page, so a 64 KiB input taking two seconds is a superlinear
path, not a slow machine. Arbitrarily deep nesting is in scope: html5ever does
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

Baisangur Yunussov
------------------

### Compiler test

**Prompt:**

> Verify that you can compile and run the project in the current git repo.

**Model:** not recorded.

**Response:** `cargo build`, `cargo clippy -- --deny warnings` and `cargo test`
passed on Baisangur's computer. The full response was not recorded.

### Round 2 review

- **Model:** Claude Opus 5.5
- **Reasoning effort:** medium

**Prompt:**

> Review the specification in fuzzing.md. Check it against translation_approach.md, the source in src/, and the tests in tests/. Report only findings about fuzzing.md: contradictions, incorrect claims about htmd, html5ever or pulldown-cmark, oracle rules that would report false positives or miss real bugs, and missing cases. Number each finding, give its location in fuzzing.md, and keep each finding to a few sentences. Do not edit any files.

**Output:**

I found 18 problems in `fuzzing.md`. I didn't run any code or edit any files.
The pulldown-cmark and html5ever behaviour I cite comes from my knowledge of
those crates, not from their sources; that matters most for #6, #12 and #13.
Findings are ordered by where they appear in `fuzzing.md`.

1. **L49–52, L222–223 — "reads Markdown exactly as the tests do" is false.**
   `tests/common/mod.rs:54` (`round_trip`) renders with
   `CommonMarkOptions::empty()` and default `Options`, not
   `TABLES | MATH` and the fuzzed options. A fuzz finding with tables, math,
   Setext or Referenced links therefore can't be reproduced "through
   `round_trip`" as triage step 1 says. Also, `fuzz/` has its own workspace and
   lockfile, so `"0.13.4"` may resolve to a newer 0.13.x unless it is pinned.
   pulldown-cmark is a library and doesn't lock pulldown-cmark-escape's
   version.

2. **L58–66 — the round-trip targets fuzz options that are lossy by design.**
   - With `preformatted_code = false`, inline-code whitespace is trimmed and
     collapsed (`code.rs:145-149`), so `<code> a </code>` fails.
   - `LinkReferenceStyle::Collapsed`/`Shortcut` use the link text as the
     label. Two links with the same text but different `href`s both resolve to
     the first definition. translation_approach.md L546–548 says these styles
     will be removed.

   Both produce false positives. Fix these options in the round-trip targets,
   or scope them out.

3. **L68–70 vs L201–205 — seeds won't decode as written.** `Unstructured`
   takes the option bytes from the front of the input, so a seed file holding
   only HTML loses its first bytes to `Options`. Seeds need an option prefix.
   "HTML in" cells also need `␣`/`⏎` decoded. The table rows give only a
   `<td>…</td>` fragment; the tests wrap it in a one-column table
   (`translation_approach_tests.rs:471-474`). Used as is, html5ever drops the
   stray `<td>`.

4. **L207–208 — the Wikipedia page also exceeds the `convert` limit.** It is
   1.44 MB, against a 64 KiB `-max_len`, so libFuzzer truncates it. Only
   `Hacker News.html` (43 KB) fits.

5. **L116–120 vs L188 — the generator mostly builds rejected inputs.** It emits
   `ul`/`ol`/`li`, but the scope rejects any DOM holding them. With about 30
   tags, 8 levels and 6 children, almost every structured input is thrown away
   unless `HTMD_FUZZ_LISTS=1`. The generator should leave lists out unless that
   variable is set.

6. **L114–127, L210–213 — generator and dictionary gaps.**
   - The text alphabet has no `;` and no letter besides `a`. Text such as
     `&lt;`/`&#1;` (escaping the links section requires), HTML-block starts
     like `<div`/`<pre`/`<![CDATA[`, and email autolinks (`@`) are
     unreachable.
   - The generator has no `tfoot` or `colgroup`, which the tables section
     discusses.
   - Neither the generator nor the dictionary has `class="language-…"`.
     `code.rs:116-128` turns it into a fence info string. I believe
     `class="x language-y"` silently loses `x`, and the fuzzer can't reach it.

7. **L152–156, rule 1 — no rule for whitespace moved out of emphasis.**
   Emphasis step 3 (translation_approach L512) moves leading and trailing
   whitespace outside the element. `emphasis.rs` strips Unicode whitespace,
   including the U+00A0 the generator emits. Rule 1 only collapses ASCII
   whitespace in place, so `<em> a</em>` and `<em> a</em>` are false
   positives.

8. **L157–160, rule 2 — it hides real bugs.**
   - Applied to both sides, it maps a body-level `<br>` and `<p><br></p>` to
     the same tree. Paragraphs row 2 and Blockquotes row 2 exist only to keep
     that `<p>`, and they could regress unseen.
   - It should apply to the output side, and only where the tables accept an
     inserted `<p>`.
   - "Inline children" is undefined: it isn't clear whether comments, `<img>`
     or a verbatim `<div>` count, and with comments that choice decides
     whether `a<!--c-->b` passes.

9. **L161–164, rule 3 — the citation doesn't hold.** "Translating HTML nodes"
   names only `<em>`/`<strong>`. translation_approach.md mentions the
   `<i>`/`<em>` merge only for Pure mode (L486–489). By the spec's own policy
   at L149–150, Faithful losing `<i>` is a gap in translation_approach.md. It
   needs a row there, not just a test.

10. **L165–168, rule 4 — the trailing-LF strip hits the wrong `<pre>`.**
    - pulldown-cmark code blocks are always `<pre><code>`, so they are never
      wrapped. A `<pre>` that rule 4 does wrap was written out verbatim as HTML
      (`pre.rs`) and never got an extra LF.
    - htmd itself removes one trailing LF from the input (`code.rs:61`). So
      `<pre><code>a⏎</code></pre>` fails unless the strip runs on both sides.
    - Rules 4 and 5 cite CommonMark/pulldown-cmark, not translation_approach.md,
      which L148–150 requires.

11. **L169–171, rule 5 — `escape_href` gives HTML source, not a value.** Its
    output contains `&amp;` and `&#x27;`, but attributes are compared after
    parsing, so every `href`/`src` with `&` or `'` fails. It also runs on every
    input `href`/`src`, though pulldown-cmark escapes only destinations it
    writes. Links and images htmd writes as HTML (extra attributes, missing
    `alt`, `title=""`) pass through verbatim, so `<img src="é">` would fail.

12. **L99 — step 5 is ill-defined.** Rules 4 and 5 are "input side" / "output
    side" only, so `N` is not one function. H1 is the output in step 4 and the
    input in step 5. Applying rule 5 to H1 again would turn `&amp;` into
    `&amp;amp;`.

13. **L138–143 — the comparison misses or mishandles some content.**
    - Only `<body>` children are compared. Comments before `<html>` or after
      `</html>` (children of Document) and after `</body>` (children of
      `html`) are ignored. htmd walks and emits them (`dom_walker.rs:31`), so
      dropping them goes unseen.
    - `<template>` content is in `template_contents`, not in its children; the
      spec doesn't say whether it is compared.
    - `&#13;` yields a real U+000D in H0. In a code block htmd can't encode it,
      and it comes back as LF. In `<pre>`, where rule 1 doesn't apply, the
      "compare equal" claim fails.

14. **L190 — the `<iframe>` exclusion contradicts translation_approach.md.**
    Paragraphs row 1 specifies `<p><iframe src="u">a</iframe></p>`, and the
    seeds include that row, yet the scope rejects any `<iframe>`.

15. **L189–191 — some exclusions are too broad.**
    - `<noscript>` is raw text only with scripting on (translation_approach
      L168). With `scripting_enabled = false` it is ordinary DOM and in scope.
    - L133–136 treats `<textarea>` like `<pre>`, so it belongs in the "inline
      context with a line ending" row, not the blanket row with
      `<script>`/`<style>`.
    - In a block context, `<script>`/`<style>` are specified type 1 HTML
      blocks.

16. **L186–193 — one unspecified case is missing from the scope.**
    translation_approach L160–166 marks two cases unspecified, and neither is
    in the table:
    - a comment, CDATA or PI holding blank lines inside a type 6 block, e.g.
      `<div><!--a⏎⏎b--></div>`;
    - a comment holding `</pre>` and then a newline inside `<pre>`.

    Rule 6 doesn't cover them: a blank line ends the block, and `&#10;` isn't
    decoded inside a comment. They will be reported as false positives.

17. **L145–177 — tables with a header row in `<tbody>`.** `table.rs:259-266`
    makes a leading `<th>` row the GFM header even when it sits in `<tbody>`,
    or in a bare `<tr>` the parser wraps. pulldown-cmark writes it back inside
    `<thead>`. translation_approach L313–319 accepts that shape, but `N` has no
    rule for table sections.
    `<table><tr><th>h</th></tr><tr><td>a</td></tr></table>` fails.

18. **L80–82 — the Referenced state isn't stored in `HtmlToMarkdown`.** It
    lives in a thread-local stack (`anchor.rs:38-40`), so "by the same
    `HtmlToMarkdown`" is the wrong test. A leak would also cross converters and
    inputs. libFuzzer runs inputs in one process, so a leak could show up on a
    later, unrelated input that `tmin` can't reproduce alone.

Mariya Belyavskaya
------------------

### Compiler test

**Prompt:**

> Verify that you can compile and run the project in the current git repo.

**Model:** not recorded.

**Response:**

I verified that the current project compiles and runs successfully. `cargo
build`, `cargo clippy -- --deny warnings`, and `cargo test` all exited with
status 0. The test suite reported 36 unit tests passed, 85 tests passed in the
first integration group, all remaining integration and doctests passed, and
one test was ignored.

The present fuzz crate also compiled successfully with `cargo build
--manifest-path fuzz/Cargo.toml`, and `cargo +nightly fuzz run fuzz_target_1
-- -runs=1` completed successfully. This exercised only the existing empty
`fuzz_target_1`; the proposed `convert`, `round_trip`, and
`round_trip_structured` targets are not present and were not tested.

**Commands executed and results:**

```text
cargo build
exit 0; Finished `dev` profile

cargo clippy -- --deny warnings
exit 0; Finished `dev` profile; no denied warnings

cargo test
exit 0; all test groups passed; 36 unit tests passed, 85 tests passed in the
first integration group, remaining integration and doctests passed, 1 ignored

cargo build --manifest-path fuzz/Cargo.toml
exit 0; Finished `dev` profile

cargo +nightly fuzz run fuzz_target_1 -- -runs=1
exit 0; empty target completed one bounded fuzz run

cargo fuzz --version
exit 0; cargo-fuzz 0.13.2

rustup toolchain list
exit 0; stable and nightly toolchains available
```

The root project and the only present fuzz target therefore compiled and ran.
The future fuzz targets and their generator, normalizer, corpus, dictionary, and
comparison oracles could not be verified because they are not present in the
current checkout.

### Rounds 3 and 4 review

- **Model:** GPT-6 Luna

The prompt and output below are from `fuzzing-review-final.md` at commit
`7498d57`, which round 5 merged into this file. Reasoning effort: medium. The
record states that this was the primary assistant's review and that no
independent second LLM was invoked.

**Prompt:**

> Review only the Fuzzing strategy section in fuzzing.md. Identify concrete technical errors, contradictions, missing cases, or reproducibility problems by checking the claims against the actual repository implementation and referenced translation specification. Do not suggest stylistic rewrites or speculative improvements. Report only actionable findings. For each finding, provide its location, explanation, and a minimal suggested correction. If no meaningful issue remains, say so.

**Iteration 3 findings and iteration 4 result:**

Round 3 — 2 findings:

1. **Fuzzing strategy opening and Layout:** the present-tense claims that a fuzzer and its files exist contradicted the repository, which has no `fuzz/` directory. Accepted; labeled the document as a proposed strategy and marked the implementation status.
2. **Dependency paragraph:** pins and the separate fuzz lockfile were stated as facts despite the missing manifest and lockfile. Accepted; recast the four versions as root-lockfile resolutions and marked the fuzz declarations unverified.

No stylistic preferences were treated as defects. The decoder and target descriptions are also unverified, but were covered by the single implementation-status finding rather than counted as additional defects.

Round 4 — 0 actionable findings. Rechecked the revised section against `Options`, the root lockfile, the translation specification and tests. The text now distinguishes required future behavior from verified implementation. No correction was needed. Requests to add further generic fuzzing guidance were rejected as speculative and outside the concrete defects requested.

**Iteration 4 output:**

> No actionable technical defect remains in the revised Fuzzing strategy section as a proposed design. The current checkout contains no `fuzz/` directory, so the described decoder, targets, generator, normalizer, seeds, dictionary, fuzz manifest, and fuzz lockfile cannot be verified against implementation. The specification now states that limitation explicitly and identifies the dependency versions as values resolved by the current root lockfile rather than claims about a missing fuzz manifest. The root `Options` declaration confirms twelve option fields, with `scripting_enabled` configured separately on the builder; the root lockfile confirms the stated versions of html5ever, markup5ever_rcdom, pulldown-cmark, and pulldown-cmark-escape. `translation_approach.md` and the existing tests support the cited design constraints around Faithful mode and the test helper's default renderer. Fuzz target compilation remains unverified because the fuzz crate and cargo-fuzz are absent.

### Round 5 review

- **Model:** GPT-5.6 Luna
- **Reasoning effort:** Not exposed by runtime
- **Review prompt:**

  > Review only the Fuzzing strategy section in fuzzing.md. Check it against the actual repository implementation, source code, tests, Cargo configuration, and translation_approach.md.
  >
  > Identify only concrete, actionable technical errors, contradictions, missing cases that affect correctness, or reproducibility problems. Do not suggest stylistic rewrites, generic fuzzing recommendations, or speculative improvements.
  >
  > Pay particular attention to whether the document distinguishes implemented functionality from proposed future functionality, whether dependency versions are accurately described, whether the documented targets and input decoding agree with the source code, and whether the comparison and normalization rules match the translation specification and tests.
  >
  > For each finding, provide its exact location, explain the technical problem, and suggest the smallest necessary correction.
  >
  > Do not invent repository files, implementation details, test results, or build results. If a detail cannot be verified from the current checkout, explicitly say so.
  >
  > If no actionable technical defects remain, state that clearly and explain briefly what was verified and what remains unverified.

- **Output:**

  The revised Fuzzing strategy section has no actionable technical defects remaining. I verified the implementation-status statement against the present checkout: `fuzz/` contains `Cargo.toml`, `Cargo.lock`, and the empty `fuzz_target_1` target, but it does not contain the proposed shared implementation, generator, normalizer, corpus, dictionary, or additional targets. I also verified the current `fuzz/Cargo.toml`: it declares only the path dependency on `htmd` and `libfuzzer-sys`, so the proposed generator and oracle dependencies remain future requirements rather than current manifest declarations. The root source, tests, Cargo configuration, root lockfile, and `translation_approach.md` support the remaining documented claims that were checked. The present fuzz target compiles and runs one iteration, but the proposed fuzzing strategy is not implemented and its future targets and oracles remain unverified.

- **Findings accepted and applied:**

  1. The implementation-status paragraph incorrectly said that `fuzz/` was absent. It now identifies the present minimal fuzz crate and distinguishes it from the proposed implementation.
  2. The dependency paragraph incorrectly said that the fuzz manifest and lockfile were absent. It now records the actual manifest declarations and distinguishes the root lockfile versions from the minimal fuzz lockfile.

  Both changes were later reverted: the `fuzz/` crate the review saw was in the
  reviewer's local checkout and is not committed to this branch.

- **Findings rejected:** None.

Arsen Alimzhan
--------------

### Compiler test

**Prompt:**

> Verify that you can compile and run the project in the current git repo.

**Model:** not recorded.

**Response:** The team branch `htmd-fuzzer-3` was downloaded as a GitHub ZIP archive. An attempt to clone it with Git failed because github.com could not be resolved in the command environment, so the downloaded source is not a Git clone. Running `cargo build` in the extracted directory returned `zsh:1: command not found: cargo`. Consequently, compilation, tests and execution were not verified on this computer. The fuzz targets also do not exist in the downloaded source. This result should not be represented as a successful build. The downloaded snapshot was from before the latest branch update.

### Round 6 review

- **Model:** Codex (GPT-6)

**Prompt:**

> Review the running and corpus portions of fuzzing.md for actionable ambiguities or unsupported claims; do not assume the proposed fuzzer exists.

**Output:**

1. **Seed use:** The strategy stores seeds in `fuzz/seeds/`, whereas working corpora are in `fuzz/corpus/`. The listed commands do not explain how the seeds reach the target corpus. Specify a reproducible initialization step for each target, preserving the 13-byte prefix and respecting the target's input limits.
2. **Timeout interpretation:** A 64 KiB input taking two seconds does not by itself prove superlinear complexity. The benchmark uses a different input, environment and workload. Treat the timeout as a finding to reproduce and investigate, and compare multiple sizes before describing complexity.

These are proposed changes; they have not been applied to the specification. The review concerns only the running and corpus portions of the existing specification.

Project 2 submission
====================

Team name: htmd fuzzer 3

The specification consists of the file [fuzzer-spec.md](fuzzer-spec.md).

Draft submission
----------------

### Baisangur Yunussov

GitHub userid: b_yunussov (also baisangur1 and Yemeth)

#### Contributions

Wrote the initial fuzzing specification, ran the round 1 review with Claude
Opus 5.5 and applied all 18 findings, then merged the team members' reviews
into one document and fixed the remaining inconsistencies.

#### Build and run test

The following summarizes LLM output from the prompt "Verify that you can build
and run the project in the current git repo."

**Model:** not recorded.

**Response:** `cargo build`, `cargo clippy -- --deny warnings` and `cargo test`
passed. The full response was not recorded.

#### Specification review rounds

| Round | Findings | Main changes made |
| ----- | -------- | ----------------- |
| 1 | 18 | Seeds-friendly option prefix; lists out of the generator; emphasis whitespace, code-block LF and table header normalized; one-sided matches for implied `<p>` and URLs; scope narrowed for iframe/noscript/textarea/script, nested blocks, template and CR added; pinned deps |

#### Last specification review

Prompting Claude Opus 5.5, medium effort, with:

> Review the specification in fuzzing.md. Check it against translation_approach.md, the source in src/, and the tests in tests/. Report only findings about fuzzing.md: contradictions, incorrect claims about htmd, html5ever or pulldown-cmark, oracle rules that would report false positives or miss real bugs, and missing cases. Number each finding, give its location in fuzzing.md, and keep each finding to a few sentences. Do not edit any files.

produced the following output:

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

### Mariya Belyavskaya

GitHub userid: m4yb3konig

#### Contributions

Ran three review rounds with GPT-6 Luna and GPT-5.6 Luna, which marked the
strategy as proposed and separated the verified root dependency versions from
the unverified fuzz-crate claims, and verified the build including a local
cargo-fuzz crate.

#### Build and run test

The following summarizes LLM output from the prompt "Verify that you can build
and run the project in the current git repo."

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

#### Specification review rounds

| Round | Findings | Main changes made |
| ----- | -------- | ----------------- |
| 1 | 2 | Marked the strategy as proposed and separated verified root dependency versions from unverified fuzz-crate claims |
| 2 | 0 | No actionable technical defects remained after checking the revised strategy against the available source and translation specification |
| 3 | 2 | Implementation status and dependency paragraph described a minimal fuzz crate; reverted after review because that crate is not committed |

#### Last specification review

Prompting GPT-5.6 Luna, reasoning effort not exposed by the runtime, with:

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

produced the following output:

The revised Fuzzing strategy section has no actionable technical defects
remaining. I verified the implementation-status statement against the present
checkout: `fuzz/` contains `Cargo.toml`, `Cargo.lock`, and the empty
`fuzz_target_1` target, but it does not contain the proposed shared
implementation, generator, normalizer, corpus, dictionary, or additional
targets. I also verified the current `fuzz/Cargo.toml`: it declares only the
path dependency on `htmd` and `libfuzzer-sys`, so the proposed generator and
oracle dependencies remain future requirements rather than current manifest
declarations. The root source, tests, Cargo configuration, root lockfile, and
`translation_approach.md` support the remaining documented claims that were
checked. The present fuzz target compiles and runs one iteration, but the
proposed fuzzing strategy is not implemented and its future targets and oracles
remain unverified.

Findings accepted and applied:

1. The implementation-status paragraph incorrectly said that `fuzz/` was
   absent. It now identifies the present minimal fuzz crate and distinguishes it
   from the proposed implementation.
2. The dependency paragraph incorrectly said that the fuzz manifest and lockfile
   were absent. It now records the actual manifest declarations and
   distinguishes the root lockfile versions from the minimal fuzz lockfile.

Both changes were later reverted: the `fuzz/` crate the review saw was in the
reviewer's local checkout and is not committed to this branch.

### Arsen Alimzhan

GitHub userid: alimzhan1709-droid

#### Contributions

Reviewed the running and corpus portions of the specification with GPT-6 (run
in Codex), finding gaps in seed initialization and timeout interpretation.

#### Build and run test

The following summarizes LLM output from the prompt "Verify that you can build
and run the project in the current git repo."

**Model:** GPT-6 Luna

**Response:** The team branch `htmd-fuzzer-3` was downloaded as a GitHub ZIP
archive. An attempt to clone it with Git failed because github.com could not be
resolved in the command environment, so the downloaded source is not a Git
clone. Running `cargo build` in the extracted directory returned
`zsh:1: command not found: cargo`. Consequently, compilation, tests and
execution were not verified on this computer. The fuzz targets also do not exist
in the downloaded source. This result should not be represented as a successful
build. The downloaded snapshot was from before the latest branch update.

#### Specification review rounds

| Round | Findings | Main changes made |
| ----- | -------- | ----------------- |
| 1 | 2 | Seed initialization from `fuzz/seeds/` into the working corpus; a single timeout no longer taken as proof of superlinear complexity |

#### Last specification review

Prompting GPT-6 Luna (run in Codex), medium effort, with:

> Review the running and corpus portions of fuzzing.md for actionable ambiguities or unsupported claims; do not assume the proposed fuzzer exists.

produced the following output:

1. **Seed use:** The strategy stores seeds in `fuzz/seeds/`, whereas working
   corpora are in `fuzz/corpus/`. The listed commands do not explain how the
   seeds reach the target corpus. Specify a reproducible initialization step for
   each target, preserving the 13-byte prefix and respecting the target's input
   limits.
2. **Timeout interpretation:** A 64 KiB input taking two seconds does not by
   itself prove superlinear complexity. The benchmark uses a different input,
   environment and workload. Treat the timeout as a finding to reproduce and
   investigate, and compare multiple sizes before describing complexity.

Final submission
----------------

### Baisangur Yunussov

GitHub userid: b_yunussov (also baisangur1 and Yemeth)

#### Contributions

Created the cargo-fuzz crate with the shared interface, implemented option decoding, `render` and the `convert` and `round_trip` targets, revised the "Layout", "Input decoding", "Targets > convert", "Targets > round_trip" and "Running" sections, and fixed the exponential emphasis serialization and the stack overflows on deep nesting which the `convert` target found.

#### Specification review rounds

| Round | Findings | Main changes made |
| ----- | -------- | ----------------- |
| 1 | 25 | Seeds loaded per target from the repository root, the false anchor-leak reasoning replaced by both conversions on equal-stack threads, step 5 options pinned, and wrong citations and claims about the option prefix fixed. |
| 2 | 1 | The `-timeout=2` reasoning ignored instrumentation, so `convert` is now built with `-O -s none --no-trace-compares`. |

#### Last specification review

Prompting Claude Opus 5.5, medium effort, with `Review only the sections "Layout", "Input decoding", "Targets > convert", "Targets > round_trip" and "Running" of fuzzer/fuzzer-spec.md. Check them for contradictions, ambiguities, and claims which are false for the htmd source code. Write numbered findings to review.md.` produced the following output; strikethrough indicates LLM output that I deemed incorrect or irrelevant.

**Review of fuzzer/fuzzer-spec.md: Layout, Input decoding, Targets > convert, Targets > round_trip, Running**

Checked against `src/`, `fuzz/`, cargo-fuzz 0.13.2, libfuzzer-sys 0.4.13, html5ever 0.39 and markup5ever_rcdom 0.39. Behavioural claims were checked by running htmd and pulldown-cmark 0.13.4 (`ENABLE_TABLES | ENABLE_MATH`) on the inputs quoted below.

##### Layout

1. **The module list does not match `fuzz/src/lib.rs`, and Triage contradicts it.** Layout says `fuzz/src/lib.rs` declares four modules: "option decoding, the HTML generator, the scope check, and the normalizer". The file actually declares `compare`, `generator`, `options`, `scope` and `text`, and defines `render` itself. No module is called normalizer, the comparison and text check are not mentioned, and the spec never says which module holds `N` and the matching rules. Triage case 1 then says to "Fix `fuzz/src/lib.rs`" for a normalizer bug, which conflicts with Layout putting the normalizer in a module under `fuzz/src/`.

2. **"The seed corpus" is described as one directory, but the seeds differ per target.** Layout gives a single `fuzz/seeds/`. Seeds and dictionary, though, makes some seeds convert-only (Hacker News, the deep-nesting files). Seeds whose tail is HTML also mean nothing to `round_trip_structured`, which reads those bytes as generator choices. The layout should say how seeds are split per target, for example `fuzz/seeds/<target>/`. See also 23.

3. ~~**The `[workspace]` statement is true but gives the wrong reason.** The root `Cargo.toml` has no `[workspace]`, so root `cargo build/clippy/test` (the only CI steps in `.github/workflows`) would never build `fuzz/` anyway. The `[workspace]` in `fuzz/Cargo.toml` stops cargo from looking for a parent workspace when building the fuzz crate; it is not what keeps CI unchanged. This is minor, but the causal "so" is wrong.~~

##### Input decoding

4. **"Byte 0" is ambiguous.** "Byte 0 always decodes to the default" can be read as "the byte at index 0" (the `heading_style` byte) as well as "a byte of value 0". The second reading is meant, so it should say "a zero byte" or "the value 0".

5. **The zero-prefix claim is false for the round-trip targets.** "a prefix of zero bytes gives `Options::default()` with scripting enabled" holds only for `convert`. In the round-trip targets the same prefix gives `translation_mode: Faithful`, `preformatted_code: true` and `link_reference_style: Full`. Of these, only `link_reference_style` matches the default; `Options::default()` has `Pure` and `false`.

6. **The `ol_number_spacing` clamp is not what `options.rs` documents.** The bullet clamp is direct: `list_marker_spacing(ul_bullet_spacing)` gives `clamp(1, 4)`. For ordered lists, `list.rs:154` clamps `ol_number_spacing + digits(highest_index) - index_str.len()`, not the field itself. So values outside 1–4 do not behave like "the clamped value" (`options.rs`'s wording). With value 0, for example, items `9.` and `10.` both get 1 space and lose alignment. The claim that out-of-range values "exercise the clamping `options.rs` documents" overstates what is exercised. It should cite `element_util.rs`/`list.rs` instead.

7. **The `preformatted_code` rationale misdescribes `code.rs` and cites the wrong file.** With `false`, `handle_inline_code` calls `trim_document_whitespace()`, which trims only. Interior whitespace is kept, so `<code>a  \t b</code>` becomes `` `a  \t b` ``; nothing is collapsed. `options.rs` only says "If true, the whitespace in inline `<code>` tags will be preserved" and documents neither trimming nor collapsing. The conclusion (`false` cannot round-trip) still holds because of the trim. The text should say "trimmed (`code.rs`)".

8. **The `link_reference_style` rationale is correct but incomplete, and the other targets inherit the problem.** I confirmed that `Shortcut` gives `[t] [t]` + `[t]: u` / `[t]: v`, which reads back with both links pointing at `u`. Separately, with `Referenced`, an empty link `<a href="x"></a>` under `Collapsed`/`Shortcut` gives `[][]`/`[]` and `[]: x`. `[]` is not a valid label, so the definition renders as the paragraph `[]: x`. That adds the letter `x`, which comes from an attribute, to the rendered text. In `convert`, which takes any options and runs the text check, this is reported as lost text even though htmd behaves as designed. The spec should exempt it or keep `Collapsed`/`Shortcut` out of the text check.

9. **"Raw targets" and "structured targets" are never defined.** The terms are used before the Targets section. From context, `convert` and `round_trip` are raw and `round_trip_structured` is structured, but the spec should say so. It also does not say whether the structured target decodes the 13-byte prefix with the round-trip rules (`fuzz/src/options.rs` has only `Target::Convert` and `Target::RoundTrip`).

##### Targets > convert

10. ~~**"returns `Err`" can never happen.** `HtmlToMarkdown::convert` is `Ok(self.tree_to_markdown(&self.html_to_tree(html)?))`, and `html_to_tree` always returns `Ok`. The condition does no harm, but it should be marked as a guard against future API changes, or dropped.~~

11. **"only a new thread starts from clean state" is false for `anchor.rs`.** `SCOPES` is a stack of frames. `on_doc_enter` pushes a fresh `AnchorScope` for every conversion, `on_doc_leave` pops it, and `build_referenced_anchor`/`append` touch only the top frame. A second conversion on the same thread therefore also starts clean. The stack is subscribed to only when `link_style == Referenced`. A frame can be left behind only if a conversion unwinds between enter and leave. libfuzzer-sys installs a panic hook that calls `process::abort()` (lib.rs:92-94), so that cannot happen within a fuzzing run either. The cross-input leak scenario the paragraph is built around has no path in the current code. The new thread is still a reasonable precaution, but the stated reason is wrong.

12. **"replaying the inputs of that run in order" cannot be done.** libFuzzer does not record the inputs it executes; it keeps only the new-coverage corpus entries and the crash artifact. `cargo fuzz tmin` re-runs a single file. There is no list of "the inputs of that run" to replay, so this fallback procedure does not exist as written.

13. **Matching the stack size is ambiguous, and "both or neither" cannot be guaranteed.** libFuzzer calls the target on the process main thread. That thread's stack comes from `RLIMIT_STACK` (usually 8 MiB) and is already partly used by libFuzzer's frames. Rust's std has no API to query a thread's stack size. `fuzz/Cargo.toml` has no `libc` dependency for `getrlimit`, and `-fork`/`-jobs` change the picture again. The spec should say how the size is obtained. Even then, the two threads start with different amounts of stack already in use, so an input close to the limit can overflow in one and not the other. Also, an overflow in the first conversion already aborts the process, so only the case "the second overflows, the first did not" matters.

14. **The text check in `convert` reports cases `translation_approach.md` accepts.** The target applies "Text" to every input, with no scope filter. I verified that it fails on:
    * `<p><span class="math math-inline">a%bc</span></p>` gives `$a$`; `bc` is lost. "Math" says "This translation loses source text, since it … replaces comments", and the scope table lists `%` in math.
    * `<xmp>a<b>c</b></xmp>` in Faithful mode gives `<xmp>a\<b>c\</b></xmp>`, which reads back as raw text `a&lt;b&gt;…`, adding the letters `lt`/`gt`. The scope table calls `<xmp>` unspecified.
    * The empty `Referenced` link from finding 8.

    "Text" says such cases are listed there with a citation, but none are. As specified, `convert` reports lossy or unspecified cases, which contradicts the introduction ("the fuzzer must not report it").

15. **Fragment parsing and document parsing disagree, and nothing exempts the case.** The text check parses `H0` as a `<body>` fragment, while `convert` parses a full document. For `<frameset>abc`, the document parser drops the text (htmd writes `""` in Pure mode and `<frameset></frameset>` in Faithful mode). The fragment parser ignores `<frameset>` and keeps `abc`. `convert` and `round_trip` both report lost text. The only exemption is for elements placed in `<head>`, and the scope table has no `<frameset>` row, so a parser-mode artifact is reported as an htmd finding.

16. ~~**The timing argument leaves out the build conditions.** `README.md` says "~16ms to convert a 1.37MB Wikipedia page on Apple M4", measured as an optimized benchmark. `cargo fuzz` builds with AddressSanitizer, coverage instrumentation and debug assertions by default, and `-timeout=2` covers the whole callback: two conversions, a thread spawn, two pulldown-cmark renders and two html5ever parses for the text check. The conclusion ("two seconds is a superlinear path") is probably still safe given roughly 2500x headroom, but it is not the comparison the sentence presents.~~

##### Targets > round_trip

17. **Step 1 contradicts the scope table and the introduction.** Out-of-scope inputs still "fail if it loses text". Several scope rows exist because htmd changes or drops text there. Math with `%` is exactly "comments are replaced, changing source text", and `<xmp>` (finding 14) adds `lt`/`gt`. `<template>` is ambiguous: "Text" says "text nodes in document order" but does not say whether `template_contents`, which are not children, are walked. If they are, `<template>abc</template>` (which htmd writes as `<template></template>`) loses `abc`. Step 1 therefore reports cases that the scope table marks as lossy or unspecified. Either the text check needs its own exemptions here, or step 1 must skip the rows which are out of scope because of text.

18. ~~**`H0` means two different things.** Here `H0` is "input `H0`", the raw string, while `round_trip_structured` says "`H0` is the parsed input, never the generator's tree". Scope parses `H0` as a full document, Comparison parses it as a fragment, and `convert` parses a full document. The spec should define `H0` once, as the HTML string fed to `convert`, and say which parse each step uses. Finding 15 shows how far the two parses can diverge.~~

19. **The options for step 5 are not stated.** `H2 = render(convert(H1))` does not say whether `convert` uses the same decoded options (Referenced, Setex, Indented and so on) and the same `scripting_enabled` as step 2. It also does not say whether `H1` goes through the scope check. pulldown-cmark can emit HTML that the scope table would reject (raw HTML blocks passed through, `<pre>` with content). As written, step 5 can report a case that step 1 would have filtered out.

20. **"redundant where step 4 is exact" is false.** Step 5 converts `H1`, not `H0`. Even when step 4 matches exactly, `H1` differs from `H0` as a string: it has inserted `<p>` wrappers, rows moved into `<thead>`, percent-encoded URLs, `start` removed and whitespace changed. So `convert(H1)` is a different conversion, which step 4 says nothing about. Step 5 is a check of its own, not a redundant one.

21. ~~**No text check for in-scope inputs.** Steps 2-5 do not include "Text" for inputs that pass the scope check, unlike `convert` and step 1. That is probably intended, since the tree comparison covers it. But Goals item 5 and the per-word `w0`, `w1`, … naming of the structured generator ("so that the text check names the word lost") assume the text check runs on in-scope round-trip inputs. The spec should say which applies.~~

22. **The `-max_len=4096` reasoning is muddled.** "the normalizer and the tree comparison are linear, but short inputs minimize to readable test cases": linearity argues for allowing long inputs, so the "but" points the wrong way. `tmin` minimizes regardless of `-max_len`. 4096 is also libFuzzer's default `max_len` when the corpus has no larger file, so the flag adds nothing unless a seed is larger.

##### Running

23. **The seed corpus is never used.** `cargo fuzz run <target>` with no corpus argument passes `fuzz/corpus/<target>` (cargo-fuzz `exec_fuzz` / `corpus_for`), a directory ignored by `fuzz/.gitignore`. None of the three commands names `fuzz/seeds/`, so the corpus described in "Seeds and dictionary" is never loaded. Each command needs the seed directory as an extra corpus argument, for example `cargo +nightly fuzz run convert fuzz/corpus/convert fuzz/seeds/convert -- …`.

24. **`-dict=fuzz/htmd.dict` depends on the working directory.** cargo-fuzz runs the target through `cargo run --manifest-path …` without changing directory, so the relative path resolves only when the command is run from the repository root. `cargo fuzz` itself also works from `fuzz/`, where the path breaks. The spec should state the working directory or use a path that does not depend on it.

25. ~~**The flags are inconsistent across targets.**~~
    * ~~Only `convert` gets `-timeout`. `round_trip` and `round_trip_structured` fall back to libFuzzer's 1200 s default, although Goals item 2 (non-termination) covers every conversion and `round_trip` converts twice.~~
    * ~~`convert` gets no `-dict`, even though the dictionary's tag names and references help raw byte mutation most.~~
    * ~~`round_trip_structured` gets no flags. Its `max_len` is then libFuzzer's default rather than a documented choice.~~
    * ~~`-rss_limit_mb=2048` equals libFuzzer's default, so it is redundant. Harmless, but it suggests a deliberate change.~~

#### Implementation review rounds

| Round | Findings | Main changes made |
| ----- | -------- | ----------------- |
| 1 | 4 | Inputs too short or not valid UTF-8 return `Corpus::Reject`, so libFuzzer keeps them out of the corpus. |
| 2 | 3 | The exponential emphasis serialization and the stack overflows on deep nesting were fixed in htmd with regression tests, and the quadratic time on deep lists was traced to instrumentation. |

#### Last implementation review

Prompting Claude Opus 5.5, medium effort, with `Review fuzz/src/lib.rs, fuzz/src/options.rs, fuzz/fuzz_targets/convert.rs and fuzz/fuzz_targets/round_trip.rs against the sections "Layout", "Input decoding", "Targets > convert", "Targets > round_trip" and "Running" of fuzzer/fuzzer-spec.md. Report only places where the code disagrees with the spec or is incorrect. Write numbered findings to review.md.` produced the following output; strikethrough indicates LLM output that I deemed incorrect or irrelevant.

**Review: fuzz crate vs. fuzzer/fuzzer-spec.md**

Scope: `fuzz/src/lib.rs`, `fuzz/src/options.rs`, `fuzz/fuzz_targets/convert.rs`, `fuzz/fuzz_targets/round_trip.rs` against "Layout", "Input decoding", "Targets > convert", "Targets > round_trip" and "Running". The crate builds on nightly and the 6 unit tests in `options.rs` pass. Field order, variant order, defaults, the `(d + b) mod n` rule, wrapping spacing, odd-byte flips, the 13-byte rejection, UTF-8 rejection, the three fixed round-trip fields, the `ENABLE_TABLES | ENABLE_MATH` renderer, and steps 1-5 of `round_trip` all match the spec. These places disagree with it:

1. ~~**Layout / Running: `fuzz/seeds/` and `fuzz/htmd.dict` are missing.** "Layout" says `fuzz/seeds/<target>/` holds each target's seed corpus and `fuzz/htmd.dict` the dictionary. Neither exists. The "Running" commands pass `fuzz/seeds/convert`, `fuzz/seeds/round_trip` and `-dict=fuzz/htmd.dict`, so they do not work as written: libFuzzer exits when a corpus directory it is given does not exist, and when the `-dict` file is missing.~~

2. ~~**convert: the text check and `render` run on libFuzzer's thread, not on the big-stack thread** (`fuzz_targets/convert.rs`, lines with `render(&first)` / `check_text`). The spec runs both conversions on threads with the same stack size so that "a deep input overflows in both or in neither", and says "arbitrarily deep nesting is in scope" and "a crash … from the `convert` target is always case 3" (an htmd finding). But `render(&first)` and `check_text` (which parses `H0` and `render(M)` with html5ever and walks both trees) run on the fuzzer's default stack, which is smaller than the 8 MiB `STACK_SIZE` and grows heavier under ASan. A deep input that htmd converts without trouble on the 8 MiB thread can then overflow in the fuzzer's own walk or in pulldown-cmark. That crash would be wrongly filed as an htmd finding. Either run the text check (and `render`) inside the same `STACK_SIZE` thread, or require `text.rs` to walk the tree without recursion.~~

3. ~~**Layout: `render` lives in `lib.rs` itself, not in a module.** "Layout" says the shared code "lives in modules under `fuzz/src/`, declared by `fuzz/src/lib.rs`". `lib.rs` defines `render`, the pulldown-cmark renderer that all targets and the text check use, directly instead of in a module. This is minor: either move it into a module (for example next to the comparison) or accept the deviation in the spec.~~

### Mariya Belyavskaya

GitHub userid: m4yb3konig

#### Contributions

(Contributions)

#### Specification review rounds

| Round | Findings | Main changes made |
| ----- | -------- | ----------------- |

#### Last specification review

(LLM and prompt used)

(LLM output, possibly with strikethrough)

#### Implementation review rounds

| Round | Findings | Main changes made |
| ----- | -------- | ----------------- |

#### Last implementation review

(LLM and prompt used)

(LLM output, possibly with strikethrough)

### Arsen Alimzhan

GitHub userid: alimzhan1709-droid

#### Contributions

Implemented the comparison with its normalizer, the text check and the scope check (`compare`, `text`, `scope`), revised the "Comparison", "Normalizer", "Text", "Scope" and "Triage" sections, and triaged the failures of all three targets into seven htmd findings.

#### Specification review rounds

| Round | Findings | Main changes made |
| ----- | -------- | ----------------- |
| 1 | 27 | Fragments parsed in quirks mode, comments taken out of inline runs, whitespace next to blocks and around emphasis normalized, text-changing scope rows exempted from the text check, and several scope rows and the Triage cases corrected. |
| 2 | 23 | Inline children, whitespace classes and the math span form defined, the end-condition and math rows tightened, and html5ever crashes assigned to the reference-bug case. |

#### Last specification review

Prompting Claude Opus 5.5, high effort, with `Review only the sections "Comparison", "Comparison > Normalizer", "Comparison > Text", "Comparison > Scope" and "Triage" of fuzzer/fuzzer-spec.md. Check them for contradictions, ambiguities, and claims which are false for the htmd source code, html5ever, pulldown-cmark or translation_approach.md. Write numbered findings in the chat; do not create a review file.` produced the following output; strikethrough indicates LLM output that I deemed incorrect or irrelevant.

I reviewed "Comparison", "Normalizer", "Text", "Scope" and "Triage" in `fuzzer/fuzzer-spec.md` against the htmd source, html5ever 0.39.0, pulldown-cmark 0.13.4 with pulldown-cmark-escape 0.11.0, and `translation_approach.md` (TA). I modified no files. Line numbers below are spec lines.

**Comparison**

1. ~~**Implied paragraphs has a weak or missing citation (L181-190).** The cited "At the document root" and "Blockquotes" rows only cover runs of `<br>`. Accepting a `<p>` around *any* inline run rests on TA's type-7 tradeoff paragraph (`<body>This is <em>really</em> important.</body>`, TA L107-115), which is not cited. For `<li>` there is no TA basis at all: TA Lists says "under development. Ignore it" (TA L348).~~

2. **"Inline child" leaves comments ambiguous (L184-186).** A comment is neither a text node nor an element, so by the definition it breaks a run. The trailing clause only implies this. Normalizer rule 1 depends on the same definition, so the spec should say it explicitly.

3. ~~**The `&#10;` sentence (L173-175) is true but hides two gaps.**~~
   - ~~Outside `<pre>`/`<textarea>`, Normalizer rule 1 already makes LF and space equal. htmd never encodes line endings in text either (`walk_text` uses `compress_whitespace`), so the sentence only matters for attributes and HTML blocks.~~
   - ~~`&#13;` behaves differently. In a link title or image alt that htmd writes as Markdown (TA Links requires `&#13;`), pulldown-cmark decodes it to a raw CR, and its `escape_html` leaves it unescaped. html5ever then reads that CR as LF. Expect mismatches the comparison will report that TA's prescription cannot fix.~~

4. ~~**The fragment parse hides `<html>`/`<body>` attributes (L171-173).** In body mode `<html>` merges its attributes into the root, which is never compared, and `<body>` is ignored in the fragment case. So what htmd does with `<body class>` or `<html lang>` can never be observed. No Scope row records this blind spot.~~

5. ~~**The URL rule cites nothing in TA (L191-196).** Its mechanics are correct: the `HREF_SAFE` table, `&amp;` and `&#x27;`. But it is a pulldown-cmark artifact and not one of TA's exceptions, which L4 says are the only source.~~

**Normalizer**

6. ~~**Its own principle (L200-203) is broken by four of its rules.**~~
   - ~~Rule 2 cites a TA row that does not exist yet; the Code blocks table (TA L250-256) has no trailing-LF row.~~
   - ~~Rule 4 cites the HTML default and pulldown-cmark behaviour, not TA, and TA Lists says ignore.~~
   - ~~Rule 5 openly erases a difference TA does not accept ("where it is not lossy").~~
   - ~~Rule 1's second clause is justified by rendering plus a quote about raw HTML inlines only.~~

   ~~Triage (L338-339) also lets a lossy finding become a rule "citing TA" without requiring the TA change first, which L203 demands.~~

7. **Rule 1's second clause has no `<pre>`/`<textarea>` exemption (L210-214).**
   - `pre`, `textarea`, `script` and `style` are type 1, so they are not inline children. Leading and trailing whitespace of their content is therefore removed. This hides, for example, a lost second leading LF (html5ever drops only the first) or trailing spaces in `<pre>`.
   - "HTML renders such whitespace identically" is false for elements that render inline but are on the type 6 list or are type 1 (`<iframe>`, `<textarea>`): `a <iframe>` and `a<iframe>` render differently.

8. **The whitespace classes are undefined.**
   - **Emphasis (rule 1):** `emphasis.rs` uses `trim_start`/`trim_end`, i.e. Unicode White_Space. That includes U+00A0, U+2003, U+3000 and U+2028, all of which the generator emits, plus VT and U+0085. If rule 1 moves only ASCII whitespace (as its first clause does), `<em> a</em>` is a false mismatch. CommonMark's "Unicode whitespace" would differ on U+2028 and VT.
   - **Emphasis, unstated:** the order for nested emphasis and whether collapsing runs again after the move.
   - **Comments (rule 5):** it says only "whitespace"; htmd uses ASCII whitespace (`is_ascii_whitespace`).

9. ~~**Rule 2 makes two false claims (L215-221).** Rule 2 applies to both sides alike, so neither does harm.~~
   - ~~"pulldown-cmark ends every code block with a line ending" is false for an empty block, which renders as `<pre><code></code></pre>`.~~
   - ~~"Any other `<pre>` goes out verbatim" is false in an inline context, where `pre.rs` writes it as a raw HTML inline with its whitespace collapsed. Also, `<pre><code>` is not always a code block: an inline context, attributes on `<pre>`, or a non-`language-` class on `<code>` all make htmd serialize it.~~

10. ~~**Rule 4 does not say which parser "parses as the integer 1" (L228).** htmd's `list.rs` does `parse::<i32>().unwrap_or(1).max(1)`, so `start="x"`, `"0"`, `"-3"` and `" 1"` all come out as 1 and pulldown-cmark omits `start`. HTML's integer rules read `start="x"` as default 1 too, but a strict reading reports it. `reversed` changes the default start (minor).~~

11. **Rule 6 does not define "math span" (L236-238).** It should be TA's form: the class is the only attribute, and the span is empty or holds one text node. "After rule 1" is redundant, and it conflicts with rule 1's `<pre>` exclusion for a math span inside `<pre>`. Rule 1's collapsing also goes further than TA step 2, which replaces each newline individually.

**Text**

12. ~~**The `<head>` exemption's reason is false (L249-252).** htmd does not drop head content: `walk_element` keeps it and `head_body_handler` walks it. In Faithful mode `<title>`, `<script>` and `<style>` are serialized with their letters intact. Only `<frameset>` drops text. As written, the exemption switches the check off for every input that starts with `<script>`, `<style>`, `<title>`, `<meta>` and so on. It also cites no TA text, contrary to L255-257.~~

13. **"The Scope rows for … raw text elements" names no row (L252-254).** Row 5 holds RAWTEXT/PLAINTEXT elements, while HTML's "raw text elements" are `script` and `style`. Row 2 (`pre`/`textarea` inline) changes only whitespace, so "TA changes the text" is false there.

14. **Some unspecified rows are not exempted, yet can lose letters.**
    - After row 4 (a type 1 block ending early) or row 3 (a type 6 block split at a blank line), the remainder is parsed as Markdown. A line like `[a]: b` then becomes a link reference definition and its letters vanish. Reporting this contradicts L4.
    - Row 6 (`<template>`): the spec does not say whether `template_contents` is read.

**Scope**

15. ~~**Rows 1-3 depend on htmd's decisions, not on the parsed DOM.** The table heading says "Parsed DOM holds", but "inline context" and "an element written as a type 6 HTML block" are translation decisions. For example, a `<p>` becomes an HTML block only when it starts with an `<iframe>` or a lone tag, and a `<table>` only when it is not GFM-shaped. The spec should say whether the checker re-derives these decisions or over-approximates (e.g. any type 6 ancestor). Inside a block written verbatim, TA says the classification "does not apply" (TA L104), so `<div><span><script>` is ambiguous.~~

16. **Row 4 omits a nested `<pre>`.** html5ever allows `<pre>` inside `<pre>`, and the inner `</pre>` ends the outer type 1 block: `<pre>a<pre>b</pre>⏎c</pre>`. TA L163-166 covers this ("type 1-5 inside type 1"). CommonMark matches end conditions case-insensitively, so a comment holding `</PRE>` should count too.

17. ~~**"Lists are in scope" (L280-282) contradicts the criteria.** It conflicts with L4 and with the Scope intro ("or does not specify"), since TA Lists says ignore. This is the same gap as the `<li>` part of finding 1.~~

18. **Row 9 is broader than TA.** Only an unescaped `%` starts a comment (TA L662; `span.rs` `math_source`), so `\%` should not trigger it. Row 8 is also wide: html5ever puts any leading `<script>`, `<style>`, `<meta>`, `<link>`, `<base>`, `<noscript>` or `<template>` into `<head>`, and whitespace or comments inside `<head>` count as children.

**Triage**

19. ~~**The seed format conflicts (L320-323 vs L287-289).** Every seed must start with the 13-byte prefix of all zeros except translation_mode. A failing input's own options differ, and the spec does not say whether to keep its prefix (breaking the Seeds rule) or reset it (losing the options that triggered the bug). Generated HTML for `round_trip` needs a prefix too, and may exceed `-max_len=4096`.~~

20. **Crash ownership contradicts itself (L324-326, L341).** A crash or hang in html5ever's `parse_document` happens inside `convert`, so L341 makes it "always case 3". Case 2 and the non-goals say html5ever bugs go upstream. Crashes or hangs in the harness's own steps are unclassified: the fragment parse, `render`, the comparison (e.g. pulldown-cmark slowness on htmd output, or recursion on deep trees).

21. ~~**Case 2's fix only works for round-trip failures.** "Add a scope entry" cannot absorb a reference bug that shows up in the text check, or in the `convert` target, which has no scope check and whose text exemptions are a closed list.~~

22. ~~**The `tests/common` helper descriptions are accurate but miss two points (L332-337).**~~
    - ~~`Options::default()` has `preformatted_code: false`, while the round-trip targets pin it to `true`. So even an all-zero prefix does not match the `round_trip` helper.~~
    - ~~`convert_faithful_options` forces Faithful mode anyway, so the only real condition is scripting being enabled (the builder default).~~

**Informational**

23. ~~**The `<i>`/`<b>` rule will produce a known mismatch at once.** Keeping `<i>` apart from `<em>` matches TA. But htmd in Faithful mode writes `<i>` as `*` and `<b>` as `**` (`italic_handler`/`bold_handler` in `element_handler/mod.rs`), which read back as `<em>`/`<strong>`. Every plain generated `<i>` or `<b>` will fail from the first run. The spec may want to say this is expected.~~

**Two htmd bugs found along the way (not spec issues)**
- `element_util.rs` `serialize_opts()` always uses `SerializeOpts::default()`, which has `scripting_enabled: true`. With scripting disabled, `<noscript>` text such as `&lt;b&gt;` is written unescaped and reads back as an element. Row 5 does not exclude this, so it will be reported, correctly.
- `list.rs` turns `start="0"` into 1 via `.max(1)`.

**Checked and found correct:**
- html5ever parses in quirks mode without a doctype, its fragment API accepts `quirks_mode`, and the fragment parser ignores doctype/`<html>`/`<head>`/`<body>`.
- pulldown-cmark always writes `<tbody>` after `</thead>`, writes `start` only when it is not 1, and percent-encodes destinations with `&amp;`/`&#x27;` as described.
- `code.rs` drops exactly one trailing LF.
- htmd's and pulldown-cmark's type 6 lists include `search`.
- In block context a comment is written as a framed type 2 block.
- `render_markdown` and `round_trip` match their descriptions.

#### Implementation review rounds

| Round | Findings | Main changes made |
| ----- | -------- | ----------------- |
| 1 | 9 | An implied paragraph wraps only a whole inline run, URL decoding and prefixed attribute names were fixed, and the math and head scope rows follow the spec exactly. |
| 2 | 4 | The list `start` attribute is read by the HTML rules for parsing integers. |

#### Last implementation review

Prompting Claude Opus 5.5, high effort, with `Review fuzz/src/compare.rs, fuzz/src/text.rs and fuzz/src/scope.rs against the sections "Comparison", "Normalizer", "Text" and "Scope" of fuzzer/fuzzer-spec.md. Report only places where the code disagrees with the spec or is incorrect, including any normalization the spec does not list. Write numbered findings to review.md.` produced the following output; strikethrough indicates LLM output that I deemed incorrect or irrelevant.

##### Review: comparison, normalizer, text and scope vs. fuzzer/fuzzer-spec.md

Scope of this review: `fuzz/src/compare.rs`, `fuzz/src/text.rs` and
`fuzz/src/scope.rs` in the working tree, checked against the spec sections
"Comparison", "Normalizer", "Text" and "Scope" (`fuzzer/fuzzer-spec.md:162-285`).
Only disagreements and incorrect code are listed. Each finding was confirmed
through the crate's public `compare`, `out_of_scope` and `check_text`, and
through the round trip htmd -> `render` with Faithful options. The crate's 39
unit tests pass.

These earlier problems are fixed and no longer reported: the `&amp;`/`&#x27;`
decoding order in "URLs", the dropped attribute prefixes, the partial-run `<p>`
in "Implied paragraphs", row 8 accepting a span holding an element or comment,
and row 7 skipping whitespace in `<head>`.

##### Scope

1. ~~**Rows 0-2 get the context wrong wherever htmd serializes a translatable~~
   ~~element as an HTML block** (`scope.rs:52-65`, `163-166`).~~
   ~~`Context::of_children` always gives a block context to `blockquote`, `ol`,~~
   ~~`ul`, `li` and the table sections, and always gives an inline context to~~
   ~~`h1`-`h6`, `p`, `td` and `th`. But htmd writes the whole element as a type 6~~
   ~~HTML block in several cases. `translation_approach.md` says that the~~
   ~~inline/block classification "does not apply to these contents":~~
   * ~~an element with extra attributes (`serialize_if_extra_attrs_or_inline!`~~
     ~~in the blockquote, list, li, heading and paragraph handlers);~~
   * ~~a paragraph which starts with a type 1-6 raw HTML inline ("Paragraphs");~~
   * ~~a table which is not GFM-shaped: one with no header row, a `<caption>`, a~~
     ~~`<tfoot>`, a header of `<td>` cells, and so on ("Tables"). `tfoot` is even~~
     ~~in the block list, although a table with a `<tfoot>` always goes out as~~
     ~~HTML.~~

   ~~As a result, row 2 never fires inside these elements, and the input is~~
   ~~reported, although the spec leaves the case unspecified. Each of the~~
   ~~following stays in scope, and its comment or style comes back as~~
   ~~`a⏎&#10;b`:~~
   * ~~`<blockquote id="x"><!--a⏎⏎b-->z</blockquote>`~~
   * ~~`<ul class="x"><li><!--a⏎⏎b-->z</li></ul>` and~~
     ~~`<ul><li id="x"><!--a⏎⏎b-->z</li></ul>`~~
   * ~~`<h1 id="x"><!--a⏎⏎b-->z</h1>` and `<p id="x"><!--a⏎⏎b-->z</p>`~~
   * ~~`<p><iframe>a</iframe><!--a⏎⏎b-->z</p>`~~
   * ~~`<table><tr><td>h</td></tr><tr><td><!--a⏎⏎b-->z</td></tr></table>`,~~
     ~~the same with a `<caption>`, and~~
     ~~`<table><tfoot><tr><td><!--a⏎⏎b-->z</td></tr></tfoot></table>`~~

   ~~Row 2 is also a "Text" exemption, so the text check goes wrong as well.~~
   ~~`<blockquote id="x"><script>a⏎⏎b</script>z</blockquote>` is reported as~~
   ~~lost text: the input gives `abz` and the output gives `a10bz`.~~

   ~~The same cause makes rows 0 and 1 too wide. They reject inputs which go out~~
   ~~inside an HTML block and round-trip unchanged:~~
   ~~`<p><script>a&lt;b</script>c</p>`, `<p><textarea>a⏎b</textarea></p>`,~~
   ~~`<table><tr><td><pre>a  b</pre></td></tr></table>` (no header row), and~~
   ~~`<table><caption>c</caption><tr><th>h</th></tr><tr><td><script>x&lt;</script></td></tr></table>`.~~
   ~~Two unit tests assert this wrong result: the textarea and table cases in~~
   ~~`preformatted_inline_with_whitespace` (`scope.rs:277-281`).~~

##### Normalizer

2. ~~**Rule 3 removes an empty `<tbody>` in every table, and ignores its~~
   ~~attributes** (`compare.rs:339-348`). The spec says "In a `<table>` with no~~
   ~~`<thead>`, a first row ... moves into a new `<thead>` before it, and an~~
   ~~empty `<tbody>` right after a `<thead>` is removed". The code runs the~~
   ~~removal on every table, after any `<thead>`, whether or not a row moved, and~~
   ~~it ignores the `<tbody>`'s attributes. The literal reading cannot be the~~
   ~~intent. pulldown-cmark ends a header-only table with `<tbody>⏎</tbody>`, so~~
   ~~if the removal ran only in tables with no `<thead>`, the output side would~~
   ~~keep its empty `<tbody>` and `<table><tr><th>h</th></tr></table>` would fail.~~
   ~~The removal is needed for an input which has a `<thead>` and no `<tbody>`.~~
   ~~As written, though, it is a normalization the spec does not list, and it~~
   ~~hides differences nothing accepts. Confirmed: each pair below matches.~~
   * ~~`<table><thead><tr><td>h</td></tr></thead><tbody></tbody></table>` (a~~
     ~~`<td>` header, so htmd writes it as HTML) against the same table without~~
     ~~the `<tbody>`.~~
   * ~~`<table><thead><tr><th>h</th></tr></thead><tbody id="x"></tbody></table>`~~
     ~~against the same table without the `<tbody>`. The attribute is lost too.~~
   * ~~`<table><tr><th>h</th></tr><tr><td>d</td></tr></table>` against an~~
     ~~output which adds an empty `<tbody>` between the `<thead>` and the body.~~

   ~~The spec should say where the removal applies. The code should then keep a~~
   ~~`<tbody>` which has attributes, and keep one which is not the table's last~~
   ~~child.~~

3. **Rule 4 uses a parser the spec does not name** (`compare.rs:292-298`).
   The spec says "a `start` attribute whose value parses as the integer 1",
   and gives HTML's default as the reason. The code applies `trim_ascii` and
   then `i64::from_str`. That accepts ` 1 ` and `+1`, which a strict Rust parse
   rejects. It also rejects `1.`, `1x` and `1. A`, which HTML's "rules for
   parsing integers" read as 1, so the list renders the same. For example,
   `<ol start="1. A"><li>a</li></ol>` is reported, and the generator can draw
   such a value. The spec should name the parser, and the code should follow
   it. With HTML's rules, the leading-whitespace and trailing-garbage handling
   both follow.

##### Text

4. ~~**The lost numbered word is misnamed or missing** (`text.rs:25-29`,~~
   ~~`50-62`). Words are taken from the joined letters and digits, so a digit~~
   ~~right after `wN`, whether in the same text node or the next one, joins the~~
   ~~word. The generator often places a digit after a word (`generator.rs:319-334`).~~
   ~~Confirmed: `check_text("<p>w1 2</p>", "<p>2</p>")` names `w12`, and~~
   ~~`<p>w3</p><p>4</p>` against `<p>4</p>` names `w34`. Neither word exists.~~
   ~~A word which is repeated or moved, not dropped, gets no name at all: `<p>w1~~
   ~~w2</p>` against `<p>w2 w1</p>` gives `None`. Goal 5 counts moved and~~
   ~~repeated text as lost, and the spec adds the words "so that the text check~~
   ~~names the word lost". Taking the words from each text node before joining~~
   ~~fixes the misnaming. Comparing the two word sequences in order would also~~
   ~~name a moved or repeated word.~~

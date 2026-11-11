Project 2 submission
====================

Team members are Baisangur Yunussov, Mariya Belyavskaya and Arsen Alimzhan. The
specification consists of the file [fuzzer-spec.md](fuzzer-spec.md).

Draft submission
----------------

### Contributions

* Baisangur Yunussov: Wrote the initial fuzzing specification (round 1): goals
  and non-goals, layout, input decoding, the `convert`, `round_trip` and
  `round_trip_structured` targets, comparison, normalizer, scope, seeds,
  triage and running. Ran the round 2 review with Claude Opus 5.5 and applied
  all 18 findings. Merged the team members' reviews into one document and fixed
  the round 5 inconsistencies.
* Mariya Belyavskaya: Ran the round 3, 4 and 5 reviews with GPT-6 Luna and
  GPT-5.6 Luna. Marked the strategy as proposed, added the implementation
  status, and separated the verified root dependency versions from the
  unverified fuzz-crate claims. Verified the build, including a local
  cargo-fuzz crate.
* Arsen Alimzhan: Ran the round 6 review of the running and corpus portions
  with Codex (GPT-6): seed initialization and timeout interpretation (proposed,
  not yet applied).

### Compiler test

The following statements summarize LLM output on each team member's computer,
based on the prompt "Verify that you can compile and run the project in the
current git repo."

#### Baisangur Yunussov

**Model:** not recorded.

**Response:** `cargo build`, `cargo clippy -- --deny warnings` and `cargo test`
passed on Baisangur's computer. The full response was not recorded.

#### Mariya Belyavskaya

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

#### Arsen Alimzhan

**Model:** not recorded.

**Response:** The team branch `htmd-fuzzer-3` was downloaded as a GitHub ZIP archive. An attempt to clone it with Git failed because github.com could not be resolved in the command environment, so the downloaded source is not a Git clone. Running `cargo build` in the extracted directory returned `zsh:1: command not found: cargo`. Consequently, compilation, tests and execution were not verified on this computer. The fuzz targets also do not exist in the downloaded source. This result should not be represented as a successful build. The downloaded snapshot was from before the latest branch update.

### Spec review rounds

| Round | Author | Model | Findings | Main changes made |
| ----- | ------ | ----- | -------- | ----------------- |
| 1 | Baisangur Yunussov | — | — | Initial specification |
| 2 | Baisangur Yunussov | Claude Opus 5.5 | 18 | Seeds-friendly option prefix; lists out of the generator; emphasis whitespace, code-block LF and table header normalized; one-sided matches for implied `<p>` and URLs; scope narrowed for iframe/noscript/textarea/script, nested blocks, template and CR added; pinned deps |
| 3 | Mariya Belyavskaya | GPT-6 Luna | 2 | Marked the strategy as proposed and separated verified root dependency versions from unverified fuzz-crate claims |
| 4 | Mariya Belyavskaya | GPT-6 Luna | 0 | No actionable technical defects remained after checking the revised strategy against the available source and translation specification |
| 5 | Mariya Belyavskaya | GPT-5.6 Luna | 2 | Implementation status and dependency paragraph described a minimal fuzz crate; reverted after review because that crate is not committed |
| 6 | Arsen Alimzhan | Codex (GPT-6) | 2 | Seed initialization and timeout interpretation; proposed, not yet applied |

### Round 2 review (Baisangur Yunussov)

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

### Rounds 3 and 4 review (Mariya Belyavskaya)

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

### Round 5 review (Mariya Belyavskaya)

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

### Last LLM review

Round 6, by Arsen Alimzhan.

- **Model:** Codex (GPT-6)

**Prompt:**

> Review the running and corpus portions of fuzzing.md for actionable ambiguities or unsupported claims; do not assume the proposed fuzzer exists.

**Output:**

1. **Seed use:** The strategy stores seeds in `fuzz/seeds/`, whereas working corpora are in `fuzz/corpus/`. The listed commands do not explain how the seeds reach the target corpus. Specify a reproducible initialization step for each target, preserving the 13-byte prefix and respecting the target's input limits.
2. **Timeout interpretation:** A 64 KiB input taking two seconds does not by itself prove superlinear complexity. The benchmark uses a different input, environment and workload. Treat the timeout as a finding to reproduce and investigate, and compare multiple sizes before describing complexity.

These are proposed changes; they have not been applied to the specification. The review concerns only the running and corpus portions of the existing specification.

---

Final submission
----------------

### Contributions

* Baisangur Yunussov:
* Mariya Belyavskaya:
* Arsen Alimzhan:

### Spec review rounds

(Table showing spec review rounds)

### Implementation review rounds

(Table showing implementation review rounds)

### Last LLM spec review

### Last LLM implementation review

# Review: fuzz artifacts vs. fuzzer/fuzzer-spec.md

Scope of this review: the spec sections "Targets > round_trip_structured"
(`fuzzer/fuzzer-spec.md:100-160`) and "Seeds and dictionary"
(`fuzzer/fuzzer-spec.md:274-306`), checked against:

* `fuzz/src/generator.rs` (the `round_trip_structured` generator),
* `fuzz/fuzz_targets/round_trip_structured.rs`,
* `fuzz/htmd.dict`,
* the seed files in `fuzz/seeds/convert` and `fuzz/seeds/round_trip`
  (produced by `fuzz/make_seeds.py`).

## Findings

1. **Stray end tag in the second neighbor template** — `fuzz/src/generator.rs:109`.

   The spec (`fuzzer/fuzzer-spec.md:151-155`, "Half of the generated trees are
   placed between neighbors which stay the same") describes the second neighbor
   as `` `b<em>e</em>`…`<em>a</em>f` `` inside a `<p>`. The generator's template
   for that case is:

   ```rust
   out.push_str("<p>b<em>e</em>");      // line 107
   serialize_nodes(&nodes, &mut out);
   out.push_str("</em>a</em>f</p>");     // line 109
   ```

   The prefix `"<p>b<em>e</em>"` already closes the `<em>` (the text `e` is its
   content). The suffix must therefore open the second neighbor with `<em>` and
   run `<em>a</em>f</p>`; instead it starts with `</em>`, so the emitted markup
   contains a stray end tag for an element which is never open. Every generated
   tree whose first byte satisfies `choices[0] % 4 == 2` therefore does not get
   the promised neighbor pair (`b<em>e</em>` … `<em>a</em>f` inside a `<p>`) but
   markup with an unbalanced `</em>`, e.g. `<p>b<em>e</em><em>x</em></em>a</em>f</p>`.
   This is a disagreement between the generator and the spec's description of
   the generated neighbors, and it puts unbalanced markup into the corpus the
   generator exists to make clean.

2. **The dictionary is missing character references the generator emits** — `fuzz/htmd.dict` and `fuzz/src/generator.rs` (`escape_text` at line 460, `escape_attr` at line 472).

   The spec (`fuzzer/fuzzer-spec.md:302-306`) says `fuzz/htmd.dict` lists the
   character references of the generator's output, among them `&amp;` and `&lt;`.
   The dictionary file does contain those two, but not `&gt;`, and it contains
   neither `&gt;` nor `&quot;`, although the generator emits them:
   `escape_text` writes `&gt;` for `>` and `escape_attr` writes `&quot;` for `"`
   in attribute values (so a destination or title holding an angle bracket or a
   quote produces those strings). The spec's own enumeration likewise stops at
   `&lt;`. The dictionary is therefore not a complete list of the character
   references the generator emits, and the round-trip target's `-dict` cannot
   suggest the missing ones.

## Checked and found consistent (no finding)

* The option prefix of every seed file (`b"\x00" * 11 + b"\x01" + b"\x00"`) and
  the Faithful/default-options decoding it selects —
  `fuzz/make_seeds.py`, `fuzz/src/options.rs`, spec `fuzzer/fuzzer-spec.md:277-282`.
* The element, attribute, math-class, comment, text and sequence sets of the
  generator against the spec's enumerations — `fuzz/src/generator.rs`,
  spec `fuzzer/fuzzer-spec.md:108-141`; the dictionary covers all of them.
* The seed sources and naming — tables from `translation_approach.md` (minus the
  Lists table), turndown cases, Hacker News and the deep-nesting files, produced
  by `fuzz/make_seeds.py` and present in the right per-target directories — spec
  `fuzzer/fuzzer-spec.md:282-300`.
* The structured target's scope-rejection counting and printing
  (`fuzz/fuzz_targets/round_trip_structured.rs`) against spec
  `fuzzer/fuzzer-spec.md:157-160`.

The unfinished `fuzz/src/scope.rs`, `fuzz/src/compare.rs` and `fuzz/src/text.rs`
were not reviewed, per instructions.

# Review: fuzz artifacts vs. `fuzzer/fuzzer-spec.md`

Scope of this review: the spec sections "Targets > round_trip_structured"
(`fuzzer/fuzzer-spec.md:100-160`) and "Seeds and dictionary"
(`fuzzer/fuzzer-spec.md:274-306`), checked against:

* `fuzz/src/generator.rs` (the `round_trip_structured` generator),
* `fuzz/fuzz_targets/round_trip_structured.rs`,
* `fuzz/htmd.dict`,
* the seed files in `fuzz/seeds/convert` and `fuzz/seeds/round_trip`.

## Findings

1. **`fuzz/htmd.dict` carries character references the spec does not list.**

   The "Seeds and dictionary" section enumerates what `fuzz/htmd.dict` holds:
   every tag name the generator emits, the attributes, the two math classes, the
   `class="language-` prefix, the comment delimiters, the character references
   `&#10;`, `&#13;`, `&amp;`, `&lt;`, `&copy;`, `&nbsp;`, `&emsp;`, and the CommonMark
   syntax characters and start-of-line sequences of the generator's text. The
   dictionary as it now stands additionally holds `"&gt;"` and `"&quot;"`, which
   are not part of that enumeration.

   The working tree differs from the committed dictionary by exactly these two
   added lines (`+&gt;` after `&lt;`, `+&quot;` after `&emsp;`), and this is the
   only uncommitted modification in the repository. The file therefore now
   disagrees with the spec's list of the dictionary's contents.

2. **`fuzz/src/generator.rs` misuses the choice stream while building attributes.**

   `build_attrs` consumes one byte per attribute for presence, and then, when the
   attribute is `class` with a non-empty value, consumes a further byte as the
   `language-a` selector:

   ```rust
   if name == "class" && let Some(extra) = self.next() && extra % 4 == 0 { ... }
   ```

   That extra byte is read from the same cursor that hands out the per-attribute
   presence bytes, so it is really the presence byte of the next attribute in
   `ATTRIBUTES`. The attribute stream therefore shifts whenever a `class` value is
   non-empty, and the "alone or after another class" behavior is selected by a byte
   which was never meant to be a selector. The generator's attribute handling no
   longer realizes the spec's "each present or absent, empty or not ... a `class`
   value may also be `language-a`, alone or after another class".

3. **`fuzz/src/generator.rs` never takes the CR-followed-by-LF branch.**

   The branch meant to implement "A CR followed by LF is one line ending. The byte
   after the CR selects the sequence, and the LF follows it, so both are consumed
   before the sequence is inserted" is dead:

   ```rust
   if ch == "\r"
       && let Some(&selector) = self.choices.get(self.pos)
       && let Some(&next) = self.choices.get(self.pos + 1)
       && next % 8 < 6
       && pick_char(next) == "\n"
   { ... }
   ```

   `next % 8 < 6` restricts `next` to small byte values whose picked character is a
   CommonMark syntax character, never a line feed, so the condition can never hold
   and the CR-LF path is never taken; the general branch then handles the CR on its
   own and inserts the sequence selected by the byte after it.

## Checked, no finding

* The generator's tag, attribute, comment, text and sequence sets against the
  enumerations of the spec (elements, attributes, math classes, sequences,
  depth and children limits, broken markup, neighbors).
* The scope-rejection counting and printing of `fuzz/fuzz_targets/round_trip_structured.rs`.
* The seed files and `fuzz/make_seeds.py`: the corpus regenerates byte-identically
  from `translation_approach.md`, `tests/html/turndown_test_index.html`,
  `examples/page-to-markdown/html/Hacker News.html` and the deep-nesting rule.

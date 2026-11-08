# Final fuzzing strategy review

## Review prompt

> Review only the Fuzzing strategy section in fuzzing.md. Identify concrete technical errors, contradictions, missing cases, or reproducibility problems by checking the claims against the actual repository implementation and referenced translation specification. Do not suggest stylistic rewrites or speculative improvements. Report only actionable findings. For each finding, provide its location, explanation, and a minimal suggested correction. If no meaningful issue remains, say so.

## Reviewer

- Model: GPT-6 Luna.
- Reasoning/effort: medium.
- Independence: this is the primary assistant's review. No independent second LLM was invoked, so this artifact does not claim multiple-model review.

## Final review output

No actionable technical defect remains in the revised Fuzzing strategy section as a proposed design. The current checkout contains no `fuzz/` directory, so the described decoder, targets, generator, normalizer, seeds, dictionary, fuzz manifest, and fuzz lockfile cannot be verified against implementation. The specification now states that limitation explicitly and identifies the dependency versions as values resolved by the current root lockfile rather than claims about a missing fuzz manifest. The root `Options` declaration confirms twelve option fields, with `scripting_enabled` configured separately on the builder; the root lockfile confirms the stated versions of html5ever, markup5ever_rcdom, pulldown-cmark, and pulldown-cmark-escape. `translation_approach.md` and the existing tests support the cited design constraints around Faithful mode and the test helper's default renderer. Fuzz target compilation remains unverified because the fuzz crate and cargo-fuzz are absent.

## Findings accepted and applied

1. **Implementation status, opening and Layout sections.** The document presented the fuzzer and its files as existing although this checkout has no `fuzz/` directory. Changed the opening and layout to describe a proposed strategy and added an explicit implementation-status note.
2. **Dependency paragraph.** The missing fuzz manifest and lockfile made the pins impossible to verify as fuzz-crate dependencies. Rephrased the versions as current root-lockfile resolutions and stated that the future fuzz manifest's declarations and resolution remain unverified.

## Findings rejected

None. Stylistic preferences and speculative additions were excluded from the review.

## Review history continuation

- Round 3 — 2 findings:
  1. **Fuzzing strategy opening and Layout:** the present-tense claims that a fuzzer and its files exist contradicted the repository, which has no `fuzz/` directory. Accepted; labeled the document as a proposed strategy and marked the implementation status.
  2. **Dependency paragraph:** pins and the separate fuzz lockfile were stated as facts despite the missing manifest and lockfile. Accepted; recast the four versions as root-lockfile resolutions and marked the fuzz declarations unverified.
  No stylistic preferences were treated as defects. The decoder and target descriptions are also unverified, but were covered by the single implementation-status finding rather than counted as additional defects.
- Round 4 — 0 actionable findings. Rechecked the revised section against `Options`, the root lockfile, the translation specification and tests. The text now distinguishes required future behavior from verified implementation. No correction was needed. Requests to add further generic fuzzing guidance were rejected as speculative and outside the concrete defects requested.

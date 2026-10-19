---
name: publish-review
description: >-
  Read convx's README, its Japanese pair, the comparison docs, and the public
  rustdoc as a stranger reads them, before crates.io publish. Use when
  preparing a release, before cargo package and cargo publish --dry-run, or
  when checking that the crate can be understood from its pages alone.
---

# Publish review

Run this on the commit that will be published, before `cargo package` and `cargo publish --dry-run`. One failure stops those commands. An agent does not run `cargo publish` without `--dry-run` (`.cursor/rules/publish.mdc`).

## Read first

- `.cursor/rules/reader-docs.mdc`
- `.cursor/rules/rust-docs.mdc`
- `.cursor/rules/spec-checks.mdc` (a promise has a test)

Do not restate those files here. Judge the pages against them.

## Pages

- `README.md` and `README.ja.md`
- `docs/comparison.md` and `docs/comparison.ja.md`
- The crate-level rustdoc (the README) and every `//!` and `///` on an item an external crate can name: read them in `target/doc/convx/` after `cargo doc --no-deps`, as docs.rs shows them, and in the source
- `Cargo.toml` `description`, `keywords`, `categories`
- `CHANGELOG.md`, the entry of the version

`docs/design.md`, `docs/design.ja.md`, `docs/roadmap.md`, `docs/bench.md`, `docs/verification.md`, and `docs/adr/` are the decision record: they keep their IDs and do not fail for them. `docs/design.ja.md` is in the Japanese inspection below.

## Mechanical scans

Run these first. A hit is read, not counted: a hit in a private item's doc or a `//` comment is not a failure.

1. **The public items.** List every item an external crate can name: the `pub use` of `src/lib.rs` and every `pub fn` in their `impl` blocks. This list is the review's checklist.
2. **Internal pointers in public rustdoc.** For each public item, its doc and the docs of its public methods: `§`, `#` followed by digits, `P` followed by digits, `R1-`, `design`, `ADR`, `roadmap`, `Grill`, `just `, and the names of private types.
3. **Examples.** For each public type: an `# Examples` block that calls each of its public methods, or the method's own block. For each public item: a code block in the README's Usage that names or calls it, in both languages.
4. **Errors.** For each public function returning `Result`: an `# Errors` section that names the variants it can return.
5. **Links.** `cargo doc --no-deps` without warning; no relative link in either README to a file outside the package.

## How to judge

Read as someone who has the crate page and has not opened this repository. Fail a page when:

- A sentence is false against the code, `Cargo.toml`, or the tests. The same wording in several pages does not make it true (reader-docs, Accuracy).
- A promise has no test that checks it.
- A reader must know an internal pointer listed in reader-docs, Stranger context.
- A public item has no rustdoc example, no README entry, or no README example; or a public `Result` has no `# Errors`.
- The README is not in the order of reader-docs, the English and Japanese sections or code blocks diverge, or an example uses `unwrap` where `?` works, or is not pasteable after the dependency block.
- The points a caller gets wrong are not stated before the first example (row-major `f64`, logical facets, `u32` indices, the 0.x policy).
- The README links relatively to a file not in the package, or embeds an image.
- A number in the comparison has no source in `docs/bench/summary.json` and the sentence does not say where it came from.
- `description` is not the README's opening claim, or is false.
- A Japanese page has not been inspected as below, the agent that edited it scored or rewrote it, Gemini reports it as a gloss of the English, or a bare word of reader-docs' vocabulary remains.

## The Japanese inspection

Through the Antigravity CLI with a Gemini model (`agy -p <prompt> --model gemini-3.8-flash-high`; the Gemini CLI is retired), on each Japanese page of reader-docs, with `.agents/skills/natural-japanese/SKILL.md` in score mode (`references/diagnose.md`). Gemini returns the score, the findings, and whether the page is a gloss of the English; it does not rewrite. When the quota of every Gemini model of that CLI is out, another of its models that did not edit the page inspects it in the same way (`--model claude-sonnet-4-6`), and the review's Issue names the model (the owner, 2026-10-19: before 0.1.1 Gemini's quota was out for five days). The agent that edited the page does not judge the result: a finding is fixed by an edit, and the page is inspected again.

## Report

For each failure: the file, the text, the rule it breaks, and why a new reader stops there. The failures are fixed in the publish review's Issue. When none remains, record the review's commit in that Issue; `cargo package` and the dry-run are then allowed.

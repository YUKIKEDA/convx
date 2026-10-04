---
name: github-closing
description: >-
  Keep a GitHub issue open when a pull request does not finish it. Use when
  writing a PR body, a commit message, gh pr create, Closes, or any text that
  names an issue number.
---

# Closing keywords

GitHub completes an issue at merge when the PR title, body, or squash commit message contains a closing verb immediately before the issue number. Negation is still a match. `This does not close #120` completed #120.

Verbs: `close`, `closes`, `closed`, `fix`, `fixes`, `fixed`, `resolve`, `resolves`, `resolved`.

## Finish the issue

Under `## Related`, one line, nothing else on it:

```text
Closes #120
```

Do not repeat that verb next to the same number anywhere else.

## Leave the issue open

Write `#120` with none of those verbs before it.

```text
The acceptance text on #120 is still unmet.
```

## Before sending

Read the title and the full body. For each issue number, decide whether this PR finishes it. If it does not, and a closing verb is the word before the number, rewrite the sentence. Then run `gh pr create`.

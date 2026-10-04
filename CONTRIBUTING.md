# Developing convx

Procedure for humans. Agent entry: [AGENTS.md](AGENTS.md). Enforcement: `.cursor/rules/`. Canonical design: [docs/design.ja.md](docs/design.ja.md). Order and status: [docs/roadmap.md](docs/roadmap.md).

## Order

Work that touches code or conventions follows this order only.

```text
Grill (when the design branches) → Issue → branch → work → PR → human review → squash merge → retrospective
```

1. **Grill** — when the API, the meaning, or a phase boundary branches. Skill: [`.agents/skills/grilling/SKILL.md`](.agents/skills/grilling/SKILL.md). Skip it when the Issue already has acceptance text and that text does not contradict [docs/design.ja.md](docs/design.ja.md).
2. **Issue** — open it from a template. Blank issues are disabled. One Issue, one PR. A change to the meaning of the spec is a `design` Issue that updates `docs/design.ja.md` and `docs/design.md` together, before any implementation Issue. Both files carry the full specification. The Japanese file is canonical when they differ.
3. **Acceptance** — write the text decided in Grill on that Issue. Do not copy it onto the roadmap. The roadmap keeps ID, title, Issue, and status.
4. **Branch** — `type/<issue-number>-<slug>` (example: `docs/12-roadmap-rows`).
5. **Work** — that Issue only. The current row is the first not-done row in [docs/roadmap.md](docs/roadmap.md).
6. **PR** — [`.github/pull_request_template.md`](.github/pull_request_template.md). `Closes #N` on its own line under `## Related`. Open it in the same turn as the branch. A human reviews and squash-merges. Agents do not merge.
7. **Retrospective** — on the merged PR, record the three points below. The next feature branch waits until they are written.

A branch stays only while it is the head of an open pull request for its Issue. After that pull request merges, the branch is unnecessary. Delete it locally and on the remote. Do not leave it. See [`.cursor/rules/workflow.mdc`](.cursor/rules/workflow.mdc).

「進めなさい」 is work on an Issue that already exists. It is not a substitute for creating an Issue or deciding scope.

## Retrospective

Write these three points as a comment on the merged PR. Write "none" when a point is empty.

1. Whether a lesson belongs in `.cursor/rules/`, both design files, or `docs/adr/`.
2. Which Issue will carry it, if any.
3. Whether to refactor. A yes opens a `refactor` Issue and that Issue is next, before the next feature. Postponing it needs a roadmap row that names when.

An agent drafts the three points and stops. Filing the Issue waits for a later message that asks for it.

Lessons about agent behavior become rules. Lessons that change the meaning of the library go to `docs/design.ja.md` and `docs/design.md` through a `design` Issue. A reason that should outlive the spec text, and is not itself the spec, becomes one file under `docs/adr/` with Status, Date, Issue, Context, and Decision. There is no retrospective log.

A correction the user gives during the work, as an ongoing constraint, is a rule edit in that same change. A lesson first noticed at this retrospective waits for the ask to file the Issue.

## Gates

Named now. The `justfile` arrives with the crate.

- `just lint` is `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
- `just test` is `cargo test`

`just bench` is not a gate until a measurement tool exists. Speedup numbers wait until the sequential core exists and Qhull has been measured. See `.cursor/rules/bench.mdc`.

## Issues

| Template   | Label           | Commit / PR type         |
| ---------- | --------------- | ------------------------ |
| `bug`      | `type:bug`      | `fix`                    |
| `feat`     | `type:feat`     | `feat`                   |
| `refactor` | `type:refactor` | `refactor`               |
| `test`     | `type:test`     | `test`                   |
| `design`   | `type:docs`     | `docs`                   |
| `task`     | `type:chore`    | `chore` / `build` / `ci` |
| `spike`    | `type:spike`    | `chore` / `test`         |

Create these labels on GitHub once. Templates apply them. Bodies are English.

Fields: Summary, Roadmap ID, Design section, Depends on, Definition of done, Notes. On a `design` Issue, Roadmap ID may be empty when the change is the specification itself. On a `spike`, Notes states what will not be kept.

## Pull requests

Title: `type(optional-scope): subject`, all English. Body: the template headings, in English. `Closes #N` on its own line under `## Related`.

## Language

Identifiers, rustdoc, commit subjects, Issue bodies, and this file are English. Japanese in the repository is [docs/design.ja.md](docs/design.ja.md) and [README.ja.md](README.ja.md). Sample `///` comments inside the Japanese design are not copied into the implementation.

## Layout of local files

`.dev/reviews/` stays in git. Other files under `.dev/` do not. Do not rewrite a review into the design's figure or math format.

This machine is not a dump for agent files. Disk is finite. A worktree, a second checkout, a throwaway crate, a downloaded tool, a profile, a `target/` directory, or a draft note is deleted in the same turn that created it. The open checkout is where the edit goes. `.dev/reviews/` is the only path under `.dev/` that stays. See [`.cursor/rules/scratch.mdc`](.cursor/rules/scratch.mdc).

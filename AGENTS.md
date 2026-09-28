# AGENTS

Entry point for agents. Procedure: [CONTRIBUTING.md](CONTRIBUTING.md). Enforcement: `.cursor/rules/`.

| Source                       | Path                                         |
| ---------------------------- | -------------------------------------------- |
| Design (canonical, Japanese) | [docs/design.ja.md](docs/design.ja.md)       |
| Design (English)             | [docs/design.md](docs/design.md)             |
| Order and status             | [docs/roadmap.md](docs/roadmap.md)           |
| How results are checked      | [docs/verification.md](docs/verification.md) |
| Why a decision was made      | [docs/adr/](docs/adr/)                       |

`docs/adr/` is created with the first ADR. `docs/conventions.md` is created when modules exist.

The current row's ID lives only in [docs/roadmap.md](docs/roadmap.md).

## Required rules

| File                                     | Contents                                                                                       |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `.cursor/rules/scope.mdc`                | Concept and requirements decide. Ease and size do not. A half build is worse than not building |
| `.cursor/rules/defer.mdc`                | A postponement names when. Do not leave one path finished and its pair unfinished              |
| `.cursor/rules/workflow.mdc`             | Issue, branch, PR, human squash merge, then retrospective                                      |
| `.cursor/rules/consent.mdc`              | A question is not a decision                                                                   |
| `.cursor/rules/git.mdc`                  | Destructive git, commit, and push need an explicit ask                                         |
| `.cursor/rules/conventional-commits.mdc` | Commit messages                                                                                |
| `.cursor/rules/pull-requests.mdc`        | PR title and body                                                                              |
| `.cursor/rules/powershell-shell.mdc`     | PowerShell syntax for git and gh                                                               |
| `.cursor/rules/record-corrections.mdc`   | Ongoing corrections become rules in the same change                                            |
| `.cursor/rules/similar-findings.mdc`     | Fix the same kind of gap together                                                              |
| `.cursor/rules/layout.mdc`               | Single crate. Do not copy the public surface into a rule                                       |
| `.cursor/rules/bench.mdc`                | Measure before speeding up. No speedup target before Qhull                                     |
| `.cursor/rules/rust.mdc`                 | Safety, Clippy, floats, no `unsafe` in this crate                                              |
| `.cursor/rules/rust-api.mdc`             | English identifiers, API Guidelines naming                                                     |
| `.cursor/rules/rust-docs.mdc`            | English rustdoc                                                                                |
| `.cursor/rules/types.mdc`                | Illegal states are types                                                                       |
| `.cursor/rules/dev-docs.mdc`             | Mermaid and LaTeX in the design spec                                                           |

Grill, when the design branches: [`.agents/skills/grilling/SKILL.md`](.agents/skills/grilling/SKILL.md).

External sources: [API Guidelines](https://rust-lang.github.io/api-guidelines/), [rustdoc book](https://doc.rust-lang.org/stable/rustdoc/how-to-write-documentation.html).

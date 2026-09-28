## Summary

<!-- What and why. English. -->

## Related

<!-- `Closes #N` on its own line when this PR finishes that Issue. N/A if none. -->

Closes #N

## Test plan

<!-- Commands and cases. -->

- [ ] `just lint`
- [ ] `just test`

## Verification

<!-- What you actually ran. N/A if not yet. -->

## Risk / Rollback

N/A

## Checklist

- [ ] PR title is Conventional Commits, in English
- [ ] Matches `AGENTS.md`, `.cursor/rules/`, and `docs/roadmap.md`
- [ ] No `unwrap` / `expect` / `panic` / `dbg` on library paths
- [ ] No `unsafe` in convx's own code
- [ ] rustdoc in English for new public API
- [ ] This PR does not leave one path optimized or supported and its pair not
- [ ] Any deferral is a roadmap row that names when, or the item is already under Intentionally out of scope
- [ ] Docs updated when behavior or workflow changed — or N/A

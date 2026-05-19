# Sync

Fetch latest `main` and rebase every local PR bookmark onto it. Scope is
PR-bearing bookmarks; unpushed local commits without a bookmark are out of
scope. Force push is forbidden; stop and ask on any conflict.

## 1. Fetch

`jj git fetch`. Any merged PR has its commits absorbed into `main@origin` and
drops out of the "ahead of main" set automatically.

## 2. Enumerate

All PR bookmarks still ahead of `main`:
`jj log -r 'bookmarks() & (main@origin..)' --no-graph -T 'bookmarks ++ "\n"'`.

Stack roots (lowest bookmark of each independent stack):
`jj log -r 'roots(bookmarks() & (main@origin..))' --no-graph -T 'bookmarks ++ "\n"'`.
A single-PR stack appears as a root with no descendants in the set.

## 3. Rebase

For each stack root: `jj rebase -s <root-commit> -d main@origin`. jj carries
descendants along, so intra-stack chain order (and each non-root bookmark's
diffbase relationship to the one below it) is preserved.

## 4. Retarget bases of new stack roots

Each root from §2 now sits directly on `main@origin`. If its PR base on GitHub
still points at a previously-merged bookmark, retarget:

1. `gh pr view <root-bookmark> --json baseRefName -q .baseRefName`.
2. If the result is not `main`, `gh pr edit <root-bookmark> --base main`.

GitHub auto-retargets when the merged bookmark's remote branch is deleted on
merge — this step catches repos where that setting is off. Non-root bookmarks in
a stack keep their existing bases; they already point at the bookmark directly
below and stay valid because the chain moved together.

## 5. Push

Push the bookmarks from §2 in batches of at most 2 per `jj git push` invocation,
passing each via its own `--bookmark <name>` flag. On non-fast-forward rejection
the bookmark's commits became immutable — switch to
[update-pr §3 follow-up](update-pr.md#3-follow-up-path) for that bookmark
instead.

## 6. Conflicts

Stop and ask on any rebase conflict; do not resolve.

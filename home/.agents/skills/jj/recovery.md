# Recovery

A command failed or the repo is in an unexpected state. Recover
non-destructively, then tell the user what changed
([surface](SKILL.md#surface-to-the-user-at-end-of-turn)).

| State                                              | Action                                                                            |
| -------------------------------------------------- | --------------------------------------------------------------------------------- |
| Detached HEAD with changes                         | Rescue into a described jj commit before anything else. Report the commit ID.     |
| Unrelated dirty file alongside the intended change | Split into a separate commit. Push only the intended commit. Report the leftover. |
| Bookmark on the wrong commit                       | Move only if fast-forward. Otherwise stop and ask.                                |
| Non-fast-forward push rejection                    | Stop and ask. Never `--force`.                                                    |
| `Commit X is immutable`                            | Switch to the [follow-up path](update-pr.md#3-follow-up-path).                    |
| `Non-tracking remote bookmark exists`              | `jj bookmark track <name>@origin`, retry the push.                                |

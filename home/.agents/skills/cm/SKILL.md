---
name: cm
description: Commit local changes and push them straight to main, for personal repos that don't use PRs.
disable-model-invocation: true
---

# Commit and push to main

For my personal repos, where I push straight to `main` without a PR. Work repos
always go through PRs (see the `jj` skill), so never run this there.

## 1. Check the repo

Allowed repos, by origin remote:

- `bayov/dotfiles`
- `hxhagai/notes`

```bash
jj git remote list | awk '$1=="origin"{print $2}'
```

When the origin is anything else, stop and ask me before going further. Don't
add a repo to this list on your own.

## 2. Pick what goes in

```bash
jj status
jj log -r 'trunk()..@' --no-graph
```

- When I name files or a topic ("cm the config edits"), commit only those
  paths. Everything else stays in the working copy.
- When I don't, commit everything. But first check the list for things that
  look like machine state or junk (caches, sync folders, `.trash`, secrets,
  huge generated dirs). Ask me about those instead of committing them.
- When the changes are clearly unrelated to each other, make one commit per
  topic.
- Earlier local commits in `trunk()..@` get pushed too, so mention them.

## 3. Write the message

Match the style of recent commits in the same repo:

```bash
jj log -r '::trunk()' -n 15 --no-graph -T 'description.first_line() ++ "\n"'
```

One line, imperative, 72 chars or less, plain ASCII (no em dashes).

## 4. Commit and push

```bash
jj commit <paths...> -m "<message>"   # leave out <paths> to commit everything
jj git fetch
jj rebase -b @- -d 'main@origin'      # only when main@origin moved past our base
jj bookmark set main -r @-
jj git push -b main
```

The push must be a fast-forward. When it's rejected, or the rebase has
conflicts, stop and tell me. Never force-push.

## 5. Report

One line: the commit message and that `main` is pushed. Mention anything left
uncommitted.

# Install & Link

## install.sh

Runs all numbered scripts in `install/` in order:

| Script                    | What it does                                                                       |
| ------------------------- | ---------------------------------------------------------------------------------- |
| `01_platform_specific.sh` | apt packages: build tools, Python, fd, ripgrep, bat, shfmt                         |
| `02_pypi_packages.sh`     | `uv`, `dprint-py`, `ipython`                                                       |
| `03_fish.sh`              | Fish 4+ (PPA), sets as default shell                                               |
| `04_fzf.sh`               | fzf fuzzy finder                                                                   |
| `05_starship.sh`          | Starship prompt                                                                    |
| `06_rust.sh`              | Rust nightly + cranelift                                                           |
| `07_cargo_packages.sh`    | `jj-cli`, `eza`, `git-delta`, `fresh-editor`, `stylua`, `sccache`, `cargo-nextest` |
| `08_nodejs.sh`            | Node.js via fnm                                                                    |
| `09_npm_packages.sh`      | `@google/gemini-cli`, `opencode-ai`, `@fission-ai/openspec`                        |
| `10_gum.sh`               | gum (terminal UI)                                                                  |
| `11_claude_code.sh`       | Claude Code CLI + Playwright Chrome                                                |

## link.py

Mirrors `home/` into `$HOME`, creating symlinks that match the directory
structure.

Each directory can contain a `.link.toml` to control how its contents are
linked:

```toml
[[link]]
linker = "spread" # symlink each child individually (not the dir itself)
# linker = "symlink"    # (default) symlink the item itself
# linker = "recursive"  # recurse without creating a .link.toml
# linker = "copy"       # copy instead of symlink
# linker = "ignore"     # skip entirely

tags = ["wsl"] # only apply on WSL (optional)
# "!tag" negates:  tags = ["!mac"]
```

Tags are detected via `link.tags.toml`. Multiple tags can be active at once; all
specified tags must match (AND semantics).

**Options**

```
--dry-run    Show what would happen without making changes
--verbose    Print every file processed
--unlink     Remove previously created symlinks
--force      Overwrite conflicting files
```

State is tracked in `.links.toml` at the repo root.

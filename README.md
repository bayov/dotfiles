Requirements
------------

- **Linux requirements**
  - Rust & Cargo — https://www.rust-lang.org/tools/install —
    `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - sccache — https://github.com/mozilla/sccache —
    `cargo install sccache --features=all`
  - uv — https://docs.astral.sh/uv/ —
    `curl -LsSf https://astral.sh/uv/install.sh | sh`
  - fnm (for Node.js) — https://fnm.vercel.app —
    `curl -o- https://fnm.vercel.app/install | bash`
  - node (using fnm) — https://nodejs.org/en/download — `fnm install 24`
  - pnpm — https://pnpm.io/ — `curl -fsSL https://get.pnpm.io/install.sh | sh -`
  - Dotter — https://github.com/SuperCuber/dotter — `cargo install dotter`
  - Jujutsu — https://jj-vcs.github.io/jj/latest/install-and-setup/ —
    `cargo install --bin jj jj-cli`
  - jj-starship — https://github.com/dmmulroy/jj-starship —
    `cargo install jj-starship`
  - gg — https://github.com/gulbanana/gg — `cargo install gg-cli`
  - Fish — https://fishshell.com/
  - Starship — https://starship.rs/ —
    `curl -sS https://starship.rs/install.sh | sh`
  - Zellij — https://zellij.dev/documentation/installation.html —
    `cargo install zellij`
  - Eza — https://github.com/eza-community/eza — `cargo install eza`
  - Batcat — https://github.com/sharkdp/bat — `sudo apt install bat`
  - Git Delta — https://github.com/dandavison/delta — `cargo install git-delta`
  - fzf — https://github.com/junegunn/fzf —
    `git clone --depth 1 https://github.com/junegunn/fzf.git ~/.fzf && ~/.fzf/install`
  - fd — https://github.com/sharkdp/fd — `sudo apt install fd-find`
  - Ripgrep — https://github.com/BurntSushi/ripgrep — `sudo apt install ripgrep`
  - Fisher — https://github.com/jorgebucaran/fisher —
    `curl -sL https://raw.githubusercontent.com/jorgebucaran/fisher/main/functions/fisher.fish | source && fisher install jorgebucaran/fisher`
  - btop — https://github.com/aristocratos/btop — `sudo apt install btop`
  - cargo-binstall — https://github.com/cargo-bins/cargo-binstall —
    `curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash`
  - cargo-update — https://github.com/nabijaczleweli/cargo-update —
    `cargo install cargo-update`
  - dprint — https://dprint.dev/ — `uv tool install dprint-py`
  - gum — https://github.com/charmbracelet/gum
  - Fresh editor — https://getfresh.dev/ — `cargo install fresh-editor`

- **Windows requirements**
  - AutoHotKey 2.0 — https://www.autohotkey.com/
  - WezTerm — https://wezterm.org/install/windows.html
  - JetBrains Mono — https://www.jetbrains.com/lp/mono/

AI Tools
--------

- Claude Code — https://docs.anthropic.com/en/docs/claude-code —
  `pnpm add -g @anthropic-ai/claude-code`
- Gemini CLI — https://github.com/google-gemini/gemini-cli —
  `pnpm add -g @google/gemini-cli`
- OpenCode — https://github.com/opencode-ai/opencode — `pnpm add -g opencode-ai`
- OpenSpec — https://github.com/fission-ai/openspec —
  `pnpm add -g @fission-ai/openspec`

Install
-------

```sh
# install everything
./install.sh

# deploy config files to their target locations
./sync.sh
```

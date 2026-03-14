Requirements
------------

- **Linux requirements**
  - Rust & Cargo — https://www.rust-lang.org/tools/install —
    `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
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
  - Neovim — https://neovim.io/ — `sudo apt install neovim`
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
  - cargo-update — https://github.com/nabijaczleweli/cargo-update — cargo
    install cargo-update`
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

Linux
-----

If regular linux (not WSL), run the following:

```sh
./run_linux.sh deploy
```

For convenience, here are commands to install all requirements:

```sh
# general pre-requisites
sudo apt-get update
sudo apt install build-essential pkg-config libssl-dev unzip

# install linux tools
sudo apt install fd-find ripgrep neovim btop

# install fzf
git clone --depth 1 https://github.com/junegunn/fzf.git ~/.fzf && ~/.fzf/install

# install fish
sudo add-apt-repository ppa:fish-shell/release-4
sudo apt update
sudo apt install fish

# set fish as default shell
chsh -s $(which fish)

# install starship
curl -sS https://starship.rs/install.sh | sh

# install rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# install cargo tools
cargo install dotter jj-cli jj-starship zellij eza bat git-delta cargo-update fresh

# install gg-cli
sudo apt install libgtk-3-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libwebkit2gtk-4.1-dev
cargo install gg-cli

# install cargo-binstall
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# install cargo-nextest
cargo binstall cargo-nextest --secure

# install fisher (fish plugin manager)
fish -c "curl -sL https://raw.githubusercontent.com/jorgebucaran/fisher/main/functions/fisher.fish | source && fisher install jorgebucaran/fisher"

# install fnm & Node.js
curl -o- https://fnm.vercel.app/install | bash
fnm install 24 # Node.js 24

# install pnpm
curl -fsSL https://get.pnpm.io/install.sh | sh -

# install uv
curl -LsSf https://astral.sh/uv/install.sh | sh

# install uv tools
uv tool install dprint-py

# install node-based tools
pnpm add -g @anthropic-ai/claude-code @google/gemini-cli opencode-ai

# install gum
sudo mkdir -p /etc/apt/keyrings
curl -fsSL https://repo.charm.sh/apt/gpg.key | sudo gpg --dearmor -o /etc/apt/keyrings/charm.gpg
echo "deb [signed-by=/etc/apt/keyrings/charm.gpg] https://repo.charm.sh/apt/ * *" | sudo tee /etc/apt/sources.list.d/charm.list
sudo apt update && sudo apt install gum
```

Windows (WSL)
-------------

Run this **from within WSL**:

```sh
./run_wsl.sh deploy
```

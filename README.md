Requirements
------------
* **Linux requirements**
  * Rust & Cargo — https://www.rust-lang.org/tools/install — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  * Dotter — https://github.com/SuperCuber/dotter — `cargo install dotter`
  * Jujustu — https://jj-vcs.github.io/jj/latest/install-and-setup/ — `cargo install --bin jj jj-cli`
  * Fish — https://fishshell.com/
  * Starship — https://starship.rs/ — `curl -sS https://starship.rs/install.sh | sh`
  * Zellij — https://zellij.dev/documentation/installation.html — `cargo install zellij`
  * Eza — https://github.com/eza-community/eza — `cargo install eza`
  * Batcat — https://github.com/sharkdp/bat — `sudo apt install bat`
* **Windows requirements**
  * AutoHotKey 2.0 — https://www.autohotkey.com/
  * WezTerm — https://wezterm.org/install/windows.html


Linux
-----
If regular linux (not WSL), run the following:
```sh
./run_linux.sh deploy
```

For convenience, here are commands to install all requiements:
```sh
# general pre-requisites
sudo apt-get update
sudo apt install build-essential pkg-config libssl-dev

# install linux tools
sudo apt install fd-find ripgrep fzf neovim

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
cargo install dotter jj-cli zellij eza bat
```


Windows (WSL)
-------------
Run this **from within WSL**:
```sh
./run_wsl.sh deploy
```

Useful tools
------------
* fzf — https://github.com/junegunn/fzf — `sudo apt install fzf`
* fdfind — https://github.com/sharkdp/fd — `sudo apt install fd-find`
* ripgrep — https://github.com/BurntSushi/ripgrep — `sudo apt install ripgrep`

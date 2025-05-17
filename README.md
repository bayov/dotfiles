Requirements
------------
* **Linux requirements**
  * Rust & Cargo — https://www.rust-lang.org/tools/install — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  * Dotter — https://github.com/SuperCuber/dotter — `cargo install dotter`
  * Jujustu — https://jj-vcs.github.io/jj/latest/install-and-setup/ — `cargo install jujutsu`
  * Fish — https://fishshell.com/
  * Starship — https://starship.rs/ — `curl -sS https://starship.rs/install.sh | sh`
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


Windows (WSL)
-------------
Run this **from within WSL**:
```sh
./run_wsl.sh deploy
```

Useful tools
------------
* fdfind — https://github.com/sharkdp/fd — `sudo apt install fd-find`
* ripgrep — https://github.com/BurntSushi/ripgrep — `sudo apt install ripgrep`

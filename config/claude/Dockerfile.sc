# Use a slightly newer Debian base for better apt packages
FROM node:22-bookworm-slim

# Set environment variables to non-interactive to avoid prompts during apt install
ENV DEBIAN_FRONTEND=noninteractive
ENV PATH="/home/node/.cargo/bin:/home/node/.local/bin:/home/node/.fzf/bin:$PATH"

# 1. Install system dependencies (cached unless apt packages change)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    unzip \
    git \
    curl \
    ca-certificates \
    python3 \
    python3-venv \
    fish \
    ripgrep \
    fd-find \
    bat \
    neovim \
    && rm -rf /var/lib/apt/lists/*

# Set up symlinks for tools that have different names in Debian
RUN ln -s /usr/bin/batcat /usr/local/bin/bat && \
    ln -s /usr/bin/fdfind /usr/local/bin/fd

# Switch to the non-root 'node' user early for tool installations
USER node
WORKDIR /home/node

# 2. Install Starship
RUN mkdir -p /home/node/.local/bin && \
    curl -sS https://starship.rs/install.sh | sh -s -- -y --bin-dir /home/node/.local/bin

# 3. Install FZF
RUN git clone --depth 1 https://github.com/junegunn/fzf.git ~/.fzf && ~/.fzf/install --all

# 4. Install Rust and Cargo-Binstall (for fast binary installation)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

# 5. Install Rust-based CLI tools via binstall (split to cache individually)
RUN /home/node/.cargo/bin/cargo binstall -y --secure jj-cli
RUN /home/node/.cargo/bin/cargo binstall -y --secure eza
RUN /home/node/.cargo/bin/cargo binstall -y --secure git-delta
RUN /home/node/.cargo/bin/cargo binstall -y --secure fresh-editor
RUN /home/node/.cargo/bin/cargo binstall -y --secure zellij

# 6. Install uv and Python tools
RUN curl -LsSf https://astral.sh/uv/install.sh | sh
RUN /home/node/.local/bin/uv tool install dprint-py

# 7. Install Claude Code (global npm installs as root, but we can install it locally or use npx, 
# or switch back to root temporarily)
USER root
RUN npm install -g @anthropic-ai/claude-code
USER node

# Change default shell to fish for the node user
USER root
RUN chsh -s /usr/bin/fish node
USER node

# Ensure required directories exist
RUN mkdir -p /home/node/.claude /home/node/.config

WORKDIR /workspace

# Default command is claude, but can be overridden to run a shell
CMD ["claude"]

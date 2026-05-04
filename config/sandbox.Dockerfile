# Use Ubuntu 24.04 as the base for a general purpose environment.
FROM ubuntu:24.04

# Set environment variables.
ENV DOCKER_SANDBOX=1
ENV DEBIAN_FRONTEND=noninteractive

# Bootstrap: Install sudo.
RUN apt-get update && \
    apt-get install -y --no-install-recommends sudo && \
    rm -rf /var/lib/apt/lists/*

# Remove the default `ubuntu` user and create a non-root user named `sandbox`.
RUN userdel -r ubuntu && \
    groupadd -g 1000 sandbox && \
    useradd -u 1000 -g 1000 -m -s /usr/bin/bash sandbox && \
    echo "sandbox ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Switch to the non-root 'sandbox' user.
USER sandbox
WORKDIR /home/sandbox

# Copy and run installer scripts one by one to optimize cache.
COPY --chown=sandbox:sandbox dev/install/sudo.sh dev/install/

COPY --chown=sandbox:sandbox dev/install/01_platform_specific.sh dev/install/
RUN bash dev/install/01_platform_specific.sh

COPY --chown=sandbox:sandbox dev/install/02_pypi_packages.sh dev/install/
RUN bash dev/install/02_pypi_packages.sh

COPY --chown=sandbox:sandbox dev/install/03_fish.sh dev/install/
RUN bash dev/install/03_fish.sh

COPY --chown=sandbox:sandbox dev/install/04_fzf.sh dev/install/
RUN bash dev/install/04_fzf.sh

COPY --chown=sandbox:sandbox dev/install/05_starship.sh dev/install/
RUN bash dev/install/05_starship.sh

COPY --chown=sandbox:sandbox dev/install/06_rust.sh dev/install/
RUN bash dev/install/06_rust.sh

COPY --chown=sandbox:sandbox dev/install/07_cargo_packages.sh dev/install/
RUN bash dev/install/07_cargo_packages.sh

COPY --chown=sandbox:sandbox dev/install/08_nodejs.sh dev/install/
RUN bash dev/install/08_nodejs.sh

COPY --chown=sandbox:sandbox dev/install/09_npm_packages.sh dev/install/
RUN bash dev/install/09_npm_packages.sh

COPY --chown=sandbox:sandbox dev/install/10_bun.sh dev/install/
RUN bash dev/install/10_bun.sh

COPY --chown=sandbox:sandbox dev/install/11_gum.sh dev/install/
RUN bash dev/install/11_gum.sh

COPY --chown=sandbox:sandbox dev/install/12_claude_code.sh dev/install/
RUN bash dev/install/12_claude_code.sh

COPY --chown=sandbox:sandbox dev/install/14_jetbrains_mono.sh dev/install/
RUN bash dev/install/14_jetbrains_mono.sh

# Default command is fish.
ENV SHELL="/usr/bin/fish"
CMD ["/usr/bin/fish"]

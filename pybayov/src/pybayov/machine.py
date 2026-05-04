"""Detect this machine's profile, via a `.machine-profile` marker at project root."""

import os
import sys
from functools import lru_cache
from pathlib import Path
from typing import Literal

DOTFILES_DIR: Path = Path(
    os.environ.get("DOTFILES_DIR", str(Path.home() / "dotfiles"))
)
MACHINE_PROFILE_FILE: Path = DOTFILES_DIR / ".machine-profile"

MachineProfile = Literal["personal", "cato"]
VALID_MACHINE_PROFILES: frozenset[str] = frozenset({"personal", "cato"})


@lru_cache(maxsize=1)
def machine_profile() -> MachineProfile:
    """Return the marker's contents. Exit with an error if missing or invalid."""
    if not MACHINE_PROFILE_FILE.exists():
        print(
            f"error: {MACHINE_PROFILE_FILE} is missing.\n"
            f"       create it containing one of: "
            f"{', '.join(sorted(VALID_MACHINE_PROFILES))}",
            file=sys.stderr,
        )
        sys.exit(1)
    content = MACHINE_PROFILE_FILE.read_text().strip()
    if content not in VALID_MACHINE_PROFILES:
        print(
            f"error: {MACHINE_PROFILE_FILE} contains {content!r}; "
            f"must be one of: {', '.join(sorted(VALID_MACHINE_PROFILES))}",
            file=sys.stderr,
        )
        sys.exit(1)
    return content  # type: ignore[return-value]

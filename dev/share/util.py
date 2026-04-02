import contextvars
from datetime import datetime
from enum import Enum
import os
from pathlib import Path
import subprocess
import sys
from typing import Optional, TextIO


# ANSI escape sequences for coloring output.
USE_COLOR = sys.stdout.isatty() or "FORCE_COLOR" in os.environ
RESET = "\033[0m" if USE_COLOR else ""
BOLD = "\033[1m" if USE_COLOR else ""
RED = "\033[31m" if USE_COLOR else ""
GREEN = "\033[32m" if USE_COLOR else ""
YELLOW = "\033[33m" if USE_COLOR else ""
CYAN = "\033[36m" if USE_COLOR else ""
MAGENTA = "\033[95m" if USE_COLOR else ""
GRAY = "\033[90m" if USE_COLOR else ""


class LogModule(Enum):
    # Verbose debug logs.
    DEBUG = "DEBUG"
    INFO = "INFO"
    WARN = "WARN"
    ERROR = "ERROR"


class Logger:
    enabled_log_modules: set[LogModule]
    use_color: bool
    show_timestamps: bool

    def __init__(
        self,
        enabled_log_modules: set[LogModule],
        show_timestamps: bool = False,
    ):
        self.enabled_log_modules = enabled_log_modules
        self.enabled_log_modules.add(LogModule.ERROR)
        self.enabled_log_modules.add(LogModule.WARN)
        self.use_color = USE_COLOR
        self.show_timestamps = show_timestamps

    def log(
        self,
        m: LogModule,
        msg: str,
        file: TextIO = sys.stdout,
        color: Optional[str] = None,
    ):
        if m not in self.enabled_log_modules:
            return

        ts = ""
        if self.show_timestamps:
            now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            ts_color = GRAY if self.use_color else ""
            ts = f"{ts_color}{now} "

        color = color or self._log_module_color(m)
        reset = RESET if self.use_color else ""

        prefix = f"{m.value:>{self._max_log_module_len()}} | "
        print(f"{ts}{color}{prefix}{msg}{reset}", file=file, flush=True)

    def debug(self, msg: str, color: Optional[str] = None):
        self.log(LogModule.DEBUG, msg, color=color)

    def info(self, msg: str, color: Optional[str] = None):
        self.log(LogModule.INFO, msg, color=color)

    def error(self, msg: str, color: Optional[str] = None):
        self.log(LogModule.ERROR, msg, file=sys.stderr, color=color)

    def warn(self, msg: str, color: Optional[str] = None):
        self.log(LogModule.WARN, msg, file=sys.stderr, color=color)

    @property
    def is_debug_enabled(self) -> bool:
        return LogModule.DEBUG in self.enabled_log_modules

    def color(self, color: str) -> str:
        if not self.use_color:
            return ""
        return color

    def _log_module_color(self, m: LogModule) -> str:
        if not self.use_color:
            return ""
        return {
            LogModule.DEBUG: GRAY,
            LogModule.ERROR: BOLD + RED,
            LogModule.WARN: BOLD + YELLOW,
            LogModule.INFO: CYAN,
        }[m]

    def _max_log_module_len(self) -> int:
        return max(len(m.value) for m in LogModule)


_logger_ctx: contextvars.ContextVar[Logger] = contextvars.ContextVar("logger")


def logger() -> Logger:
    return _logger_ctx.get()


def set_logger(l: Logger):
    _logger_ctx.set(l)


def jj_repo_root() -> Path:
    try:
        root = subprocess.check_output(
            ["jj", "workspace", "root"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        if log := _logger_ctx.get(None):
            log.debug(f"Detected jj repository root: {root}")
        return Path(root)
    except (subprocess.CalledProcessError, FileNotFoundError):
        raise ValueError("Not in a jj repository")


def git_repo_root() -> Path:
    try:
        root = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"],
            text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        if log := _logger_ctx.get(None):
            log.debug(f"Detected git repository root: {root}")
        return Path(root)
    except (subprocess.CalledProcessError, FileNotFoundError):
        raise ValueError("Not in a git repository")


def project_root() -> Path:
    try:
        return jj_repo_root()
    except ValueError:
        pass

    try:
        return git_repo_root()
    except ValueError:
        pass

    p = Path.cwd()
    while True:
        if (p / ".bayov-personal-project").exists():
            return p
        if p.parent == p:
            break
        p = p.parent

    return Path(__file__).resolve().parent.parent


def require_project_root() -> None:
    """Exits with an error if the current working directory is not `project_root()`."""
    root = project_root()
    if Path.cwd().resolve() != root:
        print(
            f"error: must be run from the project root ({root})",
            file=sys.stderr,
        )
        sys.exit(1)

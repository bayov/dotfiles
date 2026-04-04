#!/usr/bin/env python3
"""
Run a command with a spinner animation on stderr.

Usage: spinner --title <title> -- <command> [args...]
"""

from __future__ import annotations

import argparse
import dataclasses
import signal
import subprocess
import sys
import threading
import types
from collections.abc import Callable
from typing import Any, TypeVar

from pybayov import echo_box as _eb

_T = TypeVar("_T")

FPS = 60
SPINNER_FRAMES = [
    "⢀⠀",
    "⡀⠀",
    "⠄⠀",
    "⢂⠀",
    "⡂⠀",
    "⠅⠀",
    "⢃⠀",
    "⡃⠀",
    "⠍⠀",
    "⢋⠀",
    "⡋⠀",
    "⠍⠁",
    "⢋⠁",
    "⡋⠁",
    "⠍⠉",
    "⠋⠉",
    "⠋⠉",
    "⠉⠙",
    "⠉⠙",
    "⠉⠩",
    "⠈⢙",
    "⠈⡙",
    "⢈⠩",
    "⡀⢙",
    "⠄⡙",
    "⢂⠩",
    "⡂⢘",
    "⠅⡘",
    "⢃⠨",
    "⡃⢐",
    "⠍⡐",
    "⢋⠠",
    "⡋⢀",
    "⠍⡁",
    "⢋⠁",
    "⡋⠁",
    "⠍⠉",
    "⠋⠉",
    "⠋⠉",
    "⠉⠙",
    "⠉⠙",
    "⠉⠩",
    "⠈⢙",
    "⠈⡙",
    "⠈⠩",
    "⠀⢙",
    "⠀⡙",
    "⠀⠩",
    "⠀⢘",
    "⠀⡘",
    "⠀⠨",
    "⠀⢐",
    "⠀⡐",
    "⠀⠠",
    "⠀⢀",
    "⠀⡀",
]
SPINNER_FPS = 20
COMET_SPEED = 30.0  # characters per second
TEXT_COLORS = [
    (141, 176, 207),  # blue
    (180, 120, 220),  # purple
    (220, 120, 180),  # pink
]
COLOR_CYCLE_SPEED = 0.3  # cycles per second through TEXT_COLORS
WHITE_RGB = (255, 255, 255)
TAIL_LEN = 7
RESET = "\033[0m"
CLEAR_LINE = "\r\033[K"

DEFAULT_BOX_BG = _eb.DEFAULT_BG
DEFAULT_BOX_WIDTH = _eb.DEFAULT_WIDTH
DEFAULT_BOX_MARGIN = _eb.DEFAULT_MARGIN
DEFAULT_BOX_PAD_LEFT = _eb.DEFAULT_PAD_LEFT
DEFAULT_BOX_PAD_RIGHT = _eb.DEFAULT_PAD_RIGHT
DEFAULT_BOX_PAD_TOP = _eb.DEFAULT_PAD_TOP
DEFAULT_BOX_PAD_BOTTOM = _eb.DEFAULT_PAD_BOTTOM

fg_esc = _eb.fg
bg_esc = _eb.bg
lighten = _eb.lighten
ITALIC = _eb.ITALIC


def rgb(r: int, g: int, b: int) -> str:
    """Return an ANSI true-color foreground escape sequence."""
    return f"\033[38;2;{r};{g};{b}m"


def lerp_rgb(
    c1: tuple[int, int, int], c2: tuple[int, int, int], t: float
) -> tuple[int, ...]:
    """Linearly interpolate between two RGB colors."""
    return tuple(int(a + (b - a) * t) for a, b in zip(c1, c2, strict=False))


def parse_args(argv: list[str]) -> tuple[argparse.Namespace, list[str]]:
    """Parse spinner arguments and split off the wrapped command."""
    # Split on '--' to separate spinner args from command
    try:
        sep = argv.index("--")
        spinner_argv = argv[:sep]
        cmd = argv[sep + 1 :]
    except ValueError:
        spinner_argv = argv
        cmd = []

    parser = argparse.ArgumentParser(
        prog="spinner",
        usage="spinner [--title TITLE] [--box ...] -- <command> [args...]",
    )
    parser.add_argument("--title", default="")
    parser.add_argument(
        "--box", action="store_true", help="Render spinner inside a box"
    )
    parser.add_argument("--box-title", default=None, metavar="STR")
    parser.add_argument("--box-bg", default=DEFAULT_BOX_BG, metavar="COLOR")
    parser.add_argument(
        "--box-width", type=int, default=DEFAULT_BOX_WIDTH, metavar="INT"
    )
    parser.add_argument(
        "--box-margin", type=int, default=DEFAULT_BOX_MARGIN, metavar="INT"
    )
    parser.add_argument(
        "--box-margin-top", type=int, default=None, metavar="INT"
    )
    parser.add_argument(
        "--box-margin-bottom", type=int, default=None, metavar="INT"
    )
    parser.add_argument(
        "--box-pad-left", type=int, default=DEFAULT_BOX_PAD_LEFT, metavar="INT"
    )
    parser.add_argument(
        "--box-pad-right",
        type=int,
        default=DEFAULT_BOX_PAD_RIGHT,
        metavar="INT",
    )
    parser.add_argument(
        "--box-pad-top", type=int, default=DEFAULT_BOX_PAD_TOP, metavar="INT"
    )
    parser.add_argument(
        "--box-pad-bottom",
        type=int,
        default=DEFAULT_BOX_PAD_BOTTOM,
        metavar="INT",
    )
    args = parser.parse_args(spinner_argv)
    return args, cmd


def colorize_text(
    text: str, lead: int | None, base_color: tuple[int, int, int]
) -> str:
    """Colorize text with a comet-tail effect at the lead position."""
    parts: list[str] = []
    for j, ch in enumerate(text):
        dist = lead - j if lead is not None else TAIL_LEN + 1
        if 0 <= dist <= TAIL_LEN:
            color = rgb(*lerp_rgb(WHITE_RGB, base_color, dist / TAIL_LEN))
        else:
            color = rgb(*base_color)
        parts.append(f"{color}{ch}")
    return "".join(parts)


def spin(
    title: str,
    stop_event: threading.Event,
    box: _Args | None = None,
) -> None:
    """Animate spinner. Pass box args to render spinner inside a box."""
    interval = 1.0 / FPS
    frame_acc = 0.0
    slide_acc = 0.0
    color_acc = 0.0

    up = ""
    down = ""
    blank = ""
    inner_width = 0
    ml = ""
    bg_c = ""
    pad_left = 0
    pad_right = 0

    if box:
        box_bg = box.box_bg
        width = box.box_width
        pad_left = box.box_pad_left
        pad_right = box.box_pad_right
        margin = box.box_margin
        pad_bottom = box.box_pad_bottom
        _, margin_bottom = _resolve_margins(box)
        bg_c = bg_esc(box_bg)
        ml = " " * margin
        inner_width = width - pad_left - pad_right
        blank = ml + bg_c + " " * width + RESET
        # spinner line is pad_bottom + 1 (border) + margin_bottom
        # + 1 (cursor gap) lines above cursor
        lines_below = pad_bottom + margin_bottom + 2
        up = f"\033[{lines_below}A"
        down = f"\033[{lines_below}B"

    while True:
        t = color_acc % len(TEXT_COLORS)
        i = int(t)
        base_color: tuple[int, int, int] = lerp_rgb(  # type: ignore[assignment]
            TEXT_COLORS[i], TEXT_COLORS[(i + 1) % len(TEXT_COLORS)], t - i
        )
        frame = SPINNER_FRAMES[int(frame_acc) % len(SPINNER_FRAMES)]
        frame_colored = rgb(*base_color) + frame
        if title:
            title_text = " " + title
            cycle = len(title_text) + TAIL_LEN + FPS // 2
            pos = int(slide_acc) % cycle
            lead = pos if pos < len(title_text) + TAIL_LEN else None
            colored = (
                frame_colored
                + colorize_text(title_text, lead, base_color)
                + RESET
            )
        else:
            cycle = TAIL_LEN + FPS // 2
            colored = frame_colored + RESET
        text = frame + (" " + title if title else "")

        if box:
            vis = len(text)
            fill = " " * max(0, inner_width - vis)
            content = (
                f"{ml}{bg_c}{' ' * pad_left}{colored}"
                f"{bg_c}{fill}{' ' * pad_right}{RESET}"
            )
            line = f"{up}\r{content}{down}\r"
        else:
            line = f"{CLEAR_LINE} {colored}"

        sys.stderr.write(line)
        sys.stderr.flush()
        frame_acc += SPINNER_FPS / FPS
        slide_acc += COMET_SPEED / FPS
        color_acc += COLOR_CYCLE_SPEED / FPS
        if stop_event.wait(interval):
            break

    if box:
        # blank out the spinner line; box borders are already printed
        sys.stderr.write(f"{up}\r{blank}{down}\r")
    else:
        sys.stderr.write(CLEAR_LINE)
    sys.stderr.flush()


def _make_box_config(args: _Args, mt: int, mb: int) -> _eb.BoxConfig:
    return _eb.BoxConfig(
        bg_color=args.box_bg,
        width=args.box_width,
        margin_top=mt,
        margin_right=args.box_margin,
        margin_bottom=mb,
        margin_left=args.box_margin,
        pad_top=args.box_pad_top,
        pad_right=args.box_pad_right,
        pad_bottom=args.box_pad_bottom,
        pad_left=args.box_pad_left,
        title=args.box_title,
        title_bg=lighten(args.box_bg),
    )


def _resolve_margins(args: _Args) -> tuple[int, int]:
    mt = (
        args.box_margin_top
        if args.box_margin_top is not None
        else args.box_margin
    )
    mb = (
        args.box_margin_bottom
        if args.box_margin_bottom is not None
        else args.box_margin
    )
    return mt, mb


def print_box(args: _Args) -> None:
    """Print the static box frame (borders and blank lines) to stderr."""
    box_bg = args.box_bg
    width = args.box_width
    margin = args.box_margin
    mt, mb = _resolve_margins(args)
    ml = " " * margin
    blank = ml + bg_esc(box_bg) + " " * width + RESET

    if args.box_title:
        title = args.box_title
        title_bg = lighten(box_bg)
        title_vis = len(title) + 2
        title_start = (width - title_vis) // 2
        title_end = title_start + title_vis
        left_fill = fg_esc(box_bg) + "▄" * title_start
        title_cell = (
            bg_esc(title_bg)
            + fg_esc("#ffffff")
            + ITALIC
            + " "
            + title
            + " "
            + RESET
        )
        right_fill = fg_esc(box_bg) + "▄" * max(0, width - title_end) + RESET
        border_top = ml + left_fill + title_cell + right_fill
    else:
        border_top = ml + fg_esc(box_bg) + "▄" * width + RESET

    for _ in range(mt):
        sys.stderr.write("\n")
    sys.stderr.write(border_top + "\n")
    for _ in range(args.box_pad_top):
        sys.stderr.write(blank + "\n")
    sys.stderr.write(blank + "\n")  # spinner content line (animated in place)
    for _ in range(args.box_pad_bottom):
        sys.stderr.write(blank + "\n")
    sys.stderr.write(ml + fg_esc(box_bg) + "▀" * width + RESET + "\n")
    for _ in range(mb):
        sys.stderr.write("\n")
    sys.stderr.flush()


@dataclasses.dataclass
class _ThreadResult:
    value: Any = None
    exc: BaseException | None = None


@dataclasses.dataclass
class _Args:
    box_title: str | None
    box_bg: int | str
    box_width: int
    box_margin: int
    box_margin_top: int | None
    box_margin_bottom: int | None
    box_pad_left: int
    box_pad_right: int
    box_pad_top: int
    box_pad_bottom: int


def run_with_spinner(
    fn: Callable[[], _T],
    *,
    title: str = "",
    box: bool = False,
    box_title: str | None = None,
    box_bg: int | str = DEFAULT_BOX_BG,
    box_width: int = DEFAULT_BOX_WIDTH,
    box_margin: int = DEFAULT_BOX_MARGIN,
    box_margin_top: int | None = None,
    box_margin_bottom: int | None = None,
    box_pad_left: int = DEFAULT_BOX_PAD_LEFT,
    box_pad_right: int = DEFAULT_BOX_PAD_RIGHT,
    box_pad_top: int = DEFAULT_BOX_PAD_TOP,
    box_pad_bottom: int = DEFAULT_BOX_PAD_BOTTOM,
) -> _T:
    """
    Run fn() with a spinner on stderr.

    In box mode, fn's string return value is shown in the box.
    """
    show_spinner = sys.stderr.isatty()
    r = _ThreadResult()

    def target() -> None:
        try:
            r.value = fn()
        except BaseException as e:
            r.exc = e

    thread = threading.Thread(target=target)

    _a = _Args(
        box_title=box_title,
        box_bg=box_bg,
        box_width=box_width,
        box_margin=box_margin,
        box_margin_top=box_margin_top,
        box_margin_bottom=box_margin_bottom,
        box_pad_left=box_pad_left,
        box_pad_right=box_pad_right,
        box_pad_top=box_pad_top,
        box_pad_bottom=box_pad_bottom,
    )

    if show_spinner:
        if box:
            print_box(_a)

        stop = threading.Event()
        spin_t = threading.Thread(
            target=spin, args=(title, stop, _a if box else None), daemon=True
        )
        spin_t.start()
        thread.start()
        thread.join()
        stop.set()
        spin_t.join()

        if box and r.value is not None:
            mt, mb = _resolve_margins(_a)
            result_text = str(r.value).rstrip("\n")
            inner_width = _a.box_width - _a.box_pad_left - _a.box_pad_right
            lines = [
                w
                for raw in (result_text.splitlines() or [""])
                for w in _eb.word_wrap(raw, inner_width)
            ]
            total_box_lines = mt + _a.box_pad_top + _a.box_pad_bottom + mb + 3
            sys.stderr.write(f"\033[{total_box_lines}A\r\033[J")
            sys.stderr.write(
                _eb.render_box(lines, _make_box_config(_a, mt, mb)) + "\n"
            )
            sys.stderr.flush()
    else:
        thread.start()
        thread.join()

    if r.exc is not None:
        raise r.exc
    return r.value


def main() -> None:  # noqa: C901
    """Parse arguments, run the wrapped command, and animate a spinner."""
    args, cmd = parse_args(sys.argv[1:])
    if not cmd:
        sys.stderr.write(
            "spinner: error: missing command (expected ' -- <command>')\n"
        )
        sys.exit(1)

    show_spinner = sys.stderr.isatty()
    capture_stdout = show_spinner and sys.stdout.isatty()

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE if capture_stdout else sys.stdout,
        stderr=subprocess.PIPE if show_spinner else sys.stderr,
        stdin=sys.stdin,
    )

    interrupted = False

    def forward_sigint(sig: int, frame: types.FrameType | None) -> None:
        nonlocal interrupted
        interrupted = True
        proc.send_signal(signal.SIGINT)

    original_sigint = signal.getsignal(signal.SIGINT)
    signal.signal(signal.SIGINT, forward_sigint)

    if show_spinner:
        reader: threading.Thread | None = None
        captured: list[bytes] = []
        captured_err: list[bytes] = []

        assert proc.stderr is not None

        def read_stderr() -> None:
            captured_err.append(proc.stderr.read())  # type: ignore[union-attr]

        err_reader = threading.Thread(target=read_stderr, daemon=True)
        err_reader.start()

        if capture_stdout:
            assert proc.stdout is not None

            def read_stdout() -> None:
                captured.append(proc.stdout.read())  # type: ignore[union-attr]

            reader = threading.Thread(target=read_stdout, daemon=True)
            reader.start()

        _box_args = _Args(
            box_title=args.box_title,
            box_bg=args.box_bg,
            box_width=args.box_width,
            box_margin=args.box_margin,
            box_margin_top=args.box_margin_top,
            box_margin_bottom=args.box_margin_bottom,
            box_pad_left=args.box_pad_left,
            box_pad_right=args.box_pad_right,
            box_pad_top=args.box_pad_top,
            box_pad_bottom=args.box_pad_bottom,
        )
        if args.box:
            print_box(_box_args)

        stop = threading.Event()
        t = threading.Thread(
            target=spin,
            args=(args.title, stop, _box_args if args.box else None),
            daemon=True,
        )
        t.start()
        proc.wait()
        stop.set()
        t.join()
        err_reader.join()

        if proc.returncode != 0:
            if reader is not None:
                reader.join()
            if args.box:
                if interrupted:
                    box_lines = [fg_esc(11) + "interrupted" + RESET]
                else:
                    RED = "#cc4444"
                    GREY = "#888888"
                    cmd_line = fg_esc(GREY) + " ".join(cmd) + RESET
                    error_msg = (
                        fg_esc(RED)
                        + f"command exited with {proc.returncode} exit code"
                        + RESET
                    )
                    box_lines = [cmd_line, error_msg]
                mt, mb = _resolve_margins(_box_args)
                total_box_lines = (
                    mt
                    + _box_args.box_pad_top
                    + _box_args.box_pad_bottom
                    + mb
                    + 3
                )
                sys.stderr.write(f"\033[{total_box_lines}A\r\033[J")
                sys.stderr.write(
                    _eb.render_box(
                        box_lines, _make_box_config(_box_args, mt, mb)
                    )
                    + "\n"
                )
                sys.stderr.flush()
            elif captured and captured[0]:
                sys.stdout.buffer.write(captured[0])
                sys.stdout.flush()
            if captured_err and captured_err[0]:
                sys.stderr.buffer.write(captured_err[0])
                sys.stderr.flush()
            signal.signal(signal.SIGINT, original_sigint)
            sys.exit(proc.returncode)

        if args.box and capture_stdout:
            assert reader is not None
            reader.join()
            result_text = (
                (captured[0] if captured else b"")
                .decode(errors="replace")
                .rstrip("\n")
            )
            inner_width = (
                _box_args.box_width
                - _box_args.box_pad_left
                - _box_args.box_pad_right
            )
            lines: list[str] = []
            for raw in result_text.splitlines() or [""]:
                for wrapped in _eb.word_wrap(raw, inner_width):
                    lines.append(wrapped)
            # move to top of box (including margins), clear, redraw with results
            mt, mb = _resolve_margins(_box_args)
            total_box_lines = (
                mt + _box_args.box_pad_top + _box_args.box_pad_bottom + mb + 3
            )
            sys.stderr.write(f"\033[{total_box_lines}A\r\033[J")
            sys.stderr.write(
                _eb.render_box(lines, _make_box_config(_box_args, mt, mb))
                + "\n"
            )
            sys.stderr.flush()
        elif capture_stdout:
            assert reader is not None
            reader.join()
            if captured and captured[0]:
                sys.stdout.buffer.write(captured[0])
                sys.stdout.flush()
    else:
        proc.wait()

    signal.signal(signal.SIGINT, original_sigint)
    sys.exit(proc.returncode)


if __name__ == "__main__":
    main()

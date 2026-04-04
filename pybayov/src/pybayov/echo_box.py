#!/usr/bin/env python3
"""Print text in a styled box with optional title.

Usage: echo-box [TEXT] [OPTIONS]
"""

import argparse
import dataclasses
import re
import sys

RESET = "\x1b[0m"
ITALIC = "\x1b[3m"
DEFAULT_TITLE_COLOR = "#ffffff"
DEFAULT_TITLE_PADDING = 1
DEFAULT_BG = "#0f0f12"
DEFAULT_WIDTH = 80
DEFAULT_MARGIN = 1
DEFAULT_PAD_TOP = 1
DEFAULT_PAD_RIGHT = 2
DEFAULT_PAD_BOTTOM = 1
DEFAULT_PAD_LEFT = 2

ANSI_RE = re.compile(r"\x1b\[[0-9;]*[mK]")
SGR_RE = re.compile(r"\x1b\[[0-9;]*m")


@dataclasses.dataclass
class BoxConfig:
    bg_color: int | str = DEFAULT_BG
    width: int = DEFAULT_WIDTH
    margin_top: int = 0
    margin_right: int = 0
    margin_bottom: int = 0
    margin_left: int = 0
    pad_top: int = DEFAULT_PAD_TOP
    pad_right: int = DEFAULT_PAD_RIGHT
    pad_bottom: int = DEFAULT_PAD_BOTTOM
    pad_left: int = DEFAULT_PAD_LEFT
    title: str | None = None
    title_color: int | str = DEFAULT_TITLE_COLOR
    title_bg: int | str | None = None
    title_padding: int = DEFAULT_TITLE_PADDING


def strip_ansi(s: str) -> str:
    return ANSI_RE.sub("", s)


def _sgr_clears_bg(seq: str) -> bool:
    inner = seq[2:-1]  # strip ESC[ and trailing m
    if not inner:  # \x1b[m == \x1b[0m
        return True
    return any(p in ("0", "49") for p in inner.split(";"))


def restore_bg(line: str, bg_code: str) -> str:
    """Re-inject bg_code after any SGR sequence that resets the background."""
    out: list[str] = []
    pos = 0
    for m in SGR_RE.finditer(line):
        out.append(line[pos : m.end()])
        if _sgr_clears_bg(m.group()):
            out.append(bg_code)
        pos = m.end()
    out.append(line[pos:])
    return "".join(out)


def parse_color(s: str) -> int | str:
    """Return int (palette index 0-255) or hex string '#rrggbb' unchanged."""
    s = s.strip()
    if s.startswith("#"):
        return s
    return int(s)


def hex_to_rgb(color: str) -> tuple[int, int, int]:
    h = color.lstrip("#")
    return int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)


def lighten(color: int | str, amount: int = 60) -> int | str:
    if isinstance(color, int):
        return color  # can't lighten palette colors
    r, g, b = hex_to_rgb(color)
    return f"#{min(r + amount, 255):02x}{min(g + amount, 255):02x}{min(b + amount, 255):02x}"


def fg(color: int | str) -> str:
    if isinstance(color, int):
        return f"\x1b[38;5;{color}m"
    r, g, b = hex_to_rgb(color)
    return f"\x1b[38;2;{r};{g};{b}m"


def bg(color: int | str) -> str:
    if isinstance(color, int):
        return f"\x1b[48;5;{color}m"
    r, g, b = hex_to_rgb(color)
    return f"\x1b[48;2;{r};{g};{b}m"


def word_wrap(line: str, max_width: int) -> list[str]:
    result: list[str] = []
    while len(strip_ansi(line)) > max_width:
        cut = line.rfind(" ", 0, max_width)
        if cut <= 0:
            cut = max_width
        result.append(line[:cut])
        line = line[cut:].lstrip(" ")
    result.append(line)
    return result


def render_box(lines: list[str], config: BoxConfig = BoxConfig()) -> str:
    bg_code = bg(config.bg_color)
    inner_width = config.width - config.pad_left - config.pad_right
    blank = bg_code + " " * config.width + RESET
    mleft = " " * config.margin_left
    mright = " " * config.margin_right
    out: list[str] = []

    if config.title:
        title_vis = len(strip_ansi(config.title)) + 2 * config.title_padding
        title_start = (config.width - title_vis) // 2
        title_end = title_start + title_vis
        left_fill = fg(config.bg_color) + "▄" * title_start
        title_bg_safe = (
            config.title_bg if config.title_bg is not None else config.bg_color
        )
        title_cell = (
            bg(title_bg_safe)
            + fg(config.title_color)
            + ITALIC
            + " " * config.title_padding
            + config.title
            + " " * config.title_padding
            + RESET
        )
        right_fill = (
            fg(config.bg_color) + "▄" * max(0, config.width - title_end) + RESET
        )
        border_top = mleft + left_fill + title_cell + right_fill + mright
    else:
        border_top = (
            mleft + fg(config.bg_color) + "▄" * config.width + RESET + mright
        )
    border_bot = (
        mleft + fg(config.bg_color) + "▀" * config.width + RESET + mright
    )

    for _ in range(config.margin_top):
        out.append("")

    out.append(border_top)

    for _ in range(config.pad_top):
        out.append(mleft + blank + mright)

    for line in lines:
        vis = len(strip_ansi(line))
        fill = " " * max(0, inner_width - vis)
        content = restore_bg(line, bg_code)
        out.append(
            mleft
            + bg_code
            + " " * config.pad_left
            + content
            + fill
            + " " * config.pad_right
            + RESET
            + mright
        )

    for _ in range(config.pad_bottom):
        out.append(mleft + blank + mright)

    out.append(border_bot)

    for _ in range(config.margin_bottom):
        out.append("")

    return "\n".join(out)


def main():
    parser = argparse.ArgumentParser(
        prog="echo-box",
        description="Print text in a styled box with optional title.",
        epilog="Content is read from --content or stdin.",
    )
    parser.add_argument(
        "--content",
        metavar="STR",
        default=None,
        help="Box content (or pass via stdin)",
    )
    parser.add_argument("-t", "--title", metavar="STR", help="Box title")
    parser.add_argument(
        "-c",
        "--title-fg",
        "--title-color",
        dest="title_color",
        metavar="COLOR",
        type=parse_color,
        default=DEFAULT_TITLE_COLOR,
        help=f"Title foreground color (default: {DEFAULT_TITLE_COLOR})",
    )
    parser.add_argument(
        "--title-bg",
        metavar="COLOR",
        type=parse_color,
        default=None,
        help="Title background color (default: lighter version of --bg)",
    )
    parser.add_argument(
        "--title-padding",
        metavar="INT",
        type=int,
        default=DEFAULT_TITLE_PADDING,
        help=f"Title horizontal padding (default: {DEFAULT_TITLE_PADDING})",
    )
    parser.add_argument(
        "--bg",
        dest="box_bg",
        metavar="COLOR",
        type=parse_color,
        default=DEFAULT_BG,
        help=f"Box background color (default: {DEFAULT_BG})",
    )
    parser.add_argument(
        "-w",
        "--width",
        metavar="INT",
        type=int,
        default=DEFAULT_WIDTH,
        help=f"Box width (default: {DEFAULT_WIDTH})",
    )
    parser.add_argument(
        "-m",
        "--margin",
        metavar="INT",
        type=int,
        default=DEFAULT_MARGIN,
        help=f"All margins (default: {DEFAULT_MARGIN})",
    )
    parser.add_argument("--margin-top", metavar="INT", type=int, default=None)
    parser.add_argument("--margin-right", metavar="INT", type=int, default=None)
    parser.add_argument(
        "--margin-bottom", metavar="INT", type=int, default=None
    )
    parser.add_argument("--margin-left", metavar="INT", type=int, default=None)
    parser.add_argument(
        "-p",
        "--padding",
        metavar="INT",
        type=int,
        default=None,
        help=f"All padding (default: top/bottom={DEFAULT_PAD_TOP}, left/right={DEFAULT_PAD_RIGHT})",
    )
    parser.add_argument("--padding-top", metavar="INT", type=int, default=None)
    parser.add_argument(
        "--padding-right", metavar="INT", type=int, default=None
    )
    parser.add_argument(
        "--padding-bottom", metavar="INT", type=int, default=None
    )
    parser.add_argument("--padding-left", metavar="INT", type=int, default=None)

    args = parser.parse_args()

    text = args.content
    if text is None:
        if sys.stdin.isatty():
            parser.error(
                "no content provided (use --content or pipe via stdin)"
            )
        text = sys.stdin.read()

    pad = args.padding
    config = BoxConfig(
        bg_color=args.box_bg,
        width=args.width,
        margin_top=args.margin_top
        if args.margin_top is not None
        else args.margin,
        margin_right=args.margin_right
        if args.margin_right is not None
        else args.margin,
        margin_bottom=args.margin_bottom
        if args.margin_bottom is not None
        else args.margin,
        margin_left=args.margin_left
        if args.margin_left is not None
        else args.margin,
        pad_top=args.padding_top
        if args.padding_top is not None
        else (pad if pad is not None else DEFAULT_PAD_TOP),
        pad_right=args.padding_right
        if args.padding_right is not None
        else (pad if pad is not None else DEFAULT_PAD_RIGHT),
        pad_bottom=args.padding_bottom
        if args.padding_bottom is not None
        else (pad if pad is not None else DEFAULT_PAD_BOTTOM),
        pad_left=args.padding_left
        if args.padding_left is not None
        else (pad if pad is not None else DEFAULT_PAD_LEFT),
        title=args.title,
        title_color=args.title_color,
        title_bg=args.title_bg
        if args.title_bg is not None
        else lighten(args.box_bg),
        title_padding=args.title_padding,
    )

    inner_width = config.width - config.pad_left - config.pad_right
    lines: list[str] = []
    for raw_line in text.splitlines() or [""]:
        for wrapped in word_wrap(raw_line, inner_width):
            lines.append(wrapped)

    print(render_box(lines, config))


if __name__ == "__main__":
    main()

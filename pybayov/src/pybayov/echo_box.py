#!/usr/bin/env python3
"""Print text in a styled box with optional title.

Usage: echo-box [TEXT] [OPTIONS]
"""

import argparse
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


def strip_ansi(s):
    return ANSI_RE.sub("", s)


def _sgr_clears_bg(seq):
    inner = seq[2:-1]  # strip ESC[ and trailing m
    if not inner:  # \x1b[m == \x1b[0m
        return True
    return any(p in ("0", "49") for p in inner.split(";"))


def restore_bg(line, bg_code):
    """Re-inject bg_code after any SGR sequence that resets the background."""
    out = []
    pos = 0
    for m in SGR_RE.finditer(line):
        out.append(line[pos : m.end()])
        if _sgr_clears_bg(m.group()):
            out.append(bg_code)
        pos = m.end()
    out.append(line[pos:])
    return "".join(out)


def parse_color(s):
    """Return int (palette index 0-255) or hex string '#rrggbb' unchanged."""
    s = s.strip()
    if s.startswith("#"):
        return s
    return int(s)


def hex_to_rgb(color):
    h = color.lstrip("#")
    return int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)


def lighten(color, amount=60):
    if isinstance(color, int):
        return color  # can't lighten palette colors
    r, g, b = hex_to_rgb(color)
    return f"#{min(r + amount, 255):02x}{min(g + amount, 255):02x}{min(b + amount, 255):02x}"


def fg(color):
    if isinstance(color, int):
        return f"\x1b[38;5;{color}m"
    r, g, b = hex_to_rgb(color)
    return f"\x1b[38;2;{r};{g};{b}m"


def bg(color):
    if isinstance(color, int):
        return f"\x1b[48;5;{color}m"
    r, g, b = hex_to_rgb(color)
    return f"\x1b[48;2;{r};{g};{b}m"


def word_wrap(line, max_width):
    result = []
    while len(strip_ansi(line)) > max_width:
        cut = line.rfind(" ", 0, max_width)
        if cut <= 0:
            cut = max_width
        result.append(line[:cut])
        line = line[cut:].lstrip(" ")
    result.append(line)
    return result


def render_box(
    lines,
    width,
    bg_color,
    margin_top=0,
    margin_right=0,
    margin_bottom=0,
    margin_left=0,
    pad_top=DEFAULT_PAD_TOP,
    pad_right=DEFAULT_PAD_RIGHT,
    pad_bottom=DEFAULT_PAD_BOTTOM,
    pad_left=DEFAULT_PAD_LEFT,
    title=None,
    title_color=DEFAULT_TITLE_COLOR,
    title_bg=None,
    title_padding=DEFAULT_TITLE_PADDING,
):
    bg_code = bg(bg_color)
    inner_width = width - pad_left - pad_right
    blank = bg_code + " " * width + RESET
    mleft = " " * margin_left
    mright = " " * margin_right
    out = []

    if title:
        title_vis = len(strip_ansi(title)) + 2 * title_padding
        title_start = (width - title_vis) // 2
        title_end = title_start + title_vis
        left_fill = fg(bg_color) + "▄" * title_start
        title_cell = (
            bg(title_bg)
            + fg(title_color)
            + ITALIC
            + " " * title_padding
            + title
            + " " * title_padding
            + RESET
        )
        right_fill = fg(bg_color) + "▄" * max(0, width - title_end) + RESET
        border_top = mleft + left_fill + title_cell + right_fill + mright
    else:
        border_top = mleft + fg(bg_color) + "▄" * width + RESET + mright
    border_bot = mleft + fg(bg_color) + "▀" * width + RESET + mright

    for _ in range(margin_top):
        out.append("")

    out.append(border_top)

    for _ in range(pad_top):
        out.append(mleft + blank + mright)

    for line in lines:
        vis = len(strip_ansi(line))
        fill = " " * max(0, inner_width - vis)
        content = restore_bg(line, bg_code)
        out.append(
            mleft
            + bg_code
            + " " * pad_left
            + content
            + fill
            + " " * pad_right
            + RESET
            + mright
        )

    for _ in range(pad_bottom):
        out.append(mleft + blank + mright)

    out.append(border_bot)

    for _ in range(margin_bottom):
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

    mt = args.margin_top if args.margin_top is not None else args.margin
    mr = args.margin_right if args.margin_right is not None else args.margin
    mb = args.margin_bottom if args.margin_bottom is not None else args.margin
    ml = args.margin_left if args.margin_left is not None else args.margin

    pt = (
        args.padding_top
        if args.padding_top is not None
        else (args.padding if args.padding is not None else DEFAULT_PAD_TOP)
    )
    pr = (
        args.padding_right
        if args.padding_right is not None
        else (args.padding if args.padding is not None else DEFAULT_PAD_RIGHT)
    )
    pb = (
        args.padding_bottom
        if args.padding_bottom is not None
        else (args.padding if args.padding is not None else DEFAULT_PAD_BOTTOM)
    )
    pl = (
        args.padding_left
        if args.padding_left is not None
        else (args.padding if args.padding is not None else DEFAULT_PAD_LEFT)
    )

    title_bg = (
        args.title_bg if args.title_bg is not None else lighten(args.box_bg)
    )

    inner_width = args.width - pl - pr
    lines = []
    for raw_line in text.splitlines() or [""]:
        for wrapped in word_wrap(raw_line, inner_width):
            lines.append(wrapped)

    print(
        render_box(
            lines,
            args.width,
            args.box_bg,
            margin_top=mt,
            margin_right=mr,
            margin_bottom=mb,
            margin_left=ml,
            pad_top=pt,
            pad_right=pr,
            pad_bottom=pb,
            pad_left=pl,
            title=args.title,
            title_color=args.title_color,
            title_bg=title_bg,
            title_padding=args.title_padding,
        )
    )


if __name__ == "__main__":
    main()

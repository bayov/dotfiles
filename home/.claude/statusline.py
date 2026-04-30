#!/usr/bin/env python3
import sys
import json
import os


def get_status_line():
    try:
        data = json.load(sys.stdin)
    except Exception:
        return ""

    model_name = data.get("model", {}).get("display_name", "")
    cw = data.get("context_window") or {}
    used_pct = cw.get("used_percentage") or 0
    total_input = cw.get("total_input_tokens") or 0
    total_output = cw.get("total_output_tokens") or 0
    effort_level = data.get("effort", {}).get("level", "")
    cwd = data.get("workspace", {}).get("current_dir", "")

    home = os.path.expanduser("~")
    if cwd.startswith(home):
        cwd = cwd.replace(home, "~", 1)

    # ANSI Colors
    BLUE = "\033[34m"
    ORANGE = "\033[38;5;180m"
    GREY = "\033[38;5;244m"
    RED = "\033[31m"
    YELLOW = "\033[33m"
    RESET = "\033[0m"

    total_tokens = total_input + total_output

    # Context Bar logic — color by total token count
    bar_color = GREY
    if total_tokens >= 200_000:
        bar_color = RED
    elif total_tokens >= 100_000:
        bar_color = ORANGE

    # Bar construction (10 blocks)
    num_blocks = max(0, min(10, round(used_pct / 10)))
    bar = "▓" * num_blocks + "░" * (10 - num_blocks)

    # Format token counts (e.g. 12345 -> "12.3k", 1234567 -> "1.2M")
    def fmt_tokens(n):
        if n >= 1_000_000:
            return f"{n / 1_000_000:.1f}M"
        if n >= 1_000:
            return f"{n / 1_000:.1f}k"
        return str(n)

    # Order: cwd | model | effort | bar | tokens
    cwd_part = f"{BLUE}{cwd}{RESET}"
    model_part = f"{ORANGE}◆ {model_name}{RESET}"
    bar_part = f"{bar_color}{bar} {used_pct}%{RESET}"
    tokens_part = f"{bar_color}{fmt_tokens(total_tokens)} ({fmt_tokens(total_input)}↓ {fmt_tokens(total_output)}↑){RESET}"
    effort_part = f"{GREY}[{effort_level}]{RESET}" if effort_level else ""

    # Join model and effort with single space, rest with double space
    model_effort = f"{model_part} {effort_part}" if effort_part else model_part
    parts = [cwd_part, model_effort, f"{bar_part} {GREY}|{RESET} {tokens_part}"]

    return "  ".join(parts)


if __name__ == "__main__":
    print(get_status_line())

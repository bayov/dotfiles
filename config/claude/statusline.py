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
    cost = data.get("cost", {}).get("total_cost_usd") or 0.0
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

    # Context Bar logic
    bar_color = GREY
    if used_pct > 80:
        bar_color = RED
    elif used_pct > 50:
        bar_color = YELLOW

    # Bar construction (10 blocks)
    num_blocks = max(0, min(10, round(used_pct / 10)))
    bar = "▓" * num_blocks + "░" * (10 - num_blocks)

    # Order: cwd | model | bar | cost
    cwd_part = f"{BLUE}{cwd}{RESET}"
    model_part = f"{ORANGE}◆ {model_name}{RESET}"
    bar_part = f"{bar_color}{bar} {used_pct}%{RESET}"
    cost_part = f"{GREY}${cost:.2f}{RESET}"

    return f"{cwd_part}  {model_part} {bar_part} {GREY}|{RESET} {cost_part}"

if __name__ == "__main__":
    print(get_status_line())

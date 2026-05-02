#!/usr/bin/env python3
import sys
import json
import os


def parse_transcript(path):
    """Sum session tokens, collect Skill invocations, and find first/last timestamps.

    Returns (input_tokens, output_tokens, cached_tokens, skills, duration_secs).
    Streaming-aware: when entries carry `stop_reason`, only finalized rows
    (plus the latest unfinished one) are summed to avoid double-counting.
    """
    if not path or not os.path.isfile(path):
        return 0, 0, 0, [], 0

    entries = []
    has_stop_reason = False
    skills = []
    first_ts = None
    last_ts = None

    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except Exception:
                    continue

                ts = rec.get("timestamp")
                if isinstance(ts, str):
                    if first_ts is None:
                        first_ts = ts
                    last_ts = ts

                msg = rec.get("message") or {}
                if isinstance(msg.get("usage"), dict):
                    entries.append(msg)
                    if "stop_reason" in msg:
                        has_stop_reason = True

                if rec.get("type") == "assistant":
                    content = msg.get("content") or []
                    if isinstance(content, list):
                        for block in content:
                            if (
                                isinstance(block, dict)
                                and block.get("type") == "tool_use"
                                and block.get("name") == "Skill"
                            ):
                                inp = block.get("input") or {}
                                name = inp.get("skill")
                                if isinstance(name, str) and name:
                                    skills.append(name)
    except Exception:
        return 0, 0, 0, [], 0

    if has_stop_reason:
        last = len(entries) - 1
        entries = [
            m
            for i, m in enumerate(entries)
            if m.get("stop_reason")
            or (m.get("stop_reason") is None and i == last)
        ]

    inp_tok = out_tok = cached = 0
    for m in entries:
        u = m["usage"]
        inp_tok += u.get("input_tokens") or 0
        out_tok += u.get("output_tokens") or 0
        cached += u.get("cache_read_input_tokens") or 0
        cached += u.get("cache_creation_input_tokens") or 0

    duration_secs = 0
    if first_ts and last_ts:
        from datetime import datetime

        try:
            t0 = datetime.fromisoformat(first_ts.replace("Z", "+00:00"))
            t1 = datetime.fromisoformat(last_ts.replace("Z", "+00:00"))
            duration_secs = max(0, int((t1 - t0).total_seconds()))
        except Exception:
            pass

    return inp_tok, out_tok, cached, skills, duration_secs


def get_status_line():
    try:
        data = json.load(sys.stdin)
    except Exception:
        return ""

    model_name = data.get("model", {}).get("display_name", "")
    cw = data.get("context_window") or {}
    used_pct = cw.get("used_percentage") or 0
    cur = cw.get("current_usage") or {}
    # Input tokens currently in context (matches used_percentage formula)
    in_tok = (
        (cur.get("input_tokens") or 0)
        + (cur.get("cache_creation_input_tokens") or 0)
        + (cur.get("cache_read_input_tokens") or 0)
    )
    out_tok = cur.get("output_tokens") or 0
    effort_level = data.get("effort", {}).get("level", "")
    cwd = data.get("workspace", {}).get("current_dir", "")
    transcript_path = data.get("transcript_path") or ""

    home = os.path.expanduser("~")
    if cwd.startswith(home):
        cwd = cwd.replace(home, "~", 1)

    # ANSI Colors
    BLUE = "\033[34m"
    ORANGE = "\033[38;5;180m"
    GREY = "\033[38;5;244m"
    MAGENTA = "\033[35m"
    RED = "\033[31m"
    YELLOW = "\033[33m"
    RESET = "\033[0m"

    # Color by input tokens in context (the metric the bar's percentage reflects)
    bar_color = GREY
    if in_tok >= 200_000:
        bar_color = RED
    elif in_tok >= 100_000:
        bar_color = ORANGE

    # Bar construction (10 blocks)
    num_blocks = max(0, min(10, round(used_pct / 10)))
    bar = "▓" * num_blocks + "░" * (10 - num_blocks)

    # Format token counts: no decimals for k, one decimal for M (e.g. 55k, 1.1M)
    def fmt_tokens(n):
        if n >= 1_000_000:
            return f"{n / 1_000_000:.1f}M"
        if n >= 1_000:
            return f"{round(n / 1_000)}k"
        return str(n)

    def fmt_duration(secs):
        h, rem = divmod(secs, 3600)
        m = rem // 60
        if h and m:
            return f"{h}h{m:02d}m"
        if h:
            return f"{h}h"
        return f"{m}m"

    sess_in, sess_out, sess_cached, skills, sess_secs = parse_transcript(
        transcript_path
    )
    sess_total = sess_in + sess_out + sess_cached

    # Order: cwd | model | effort | bar | context tokens | session tokens
    cwd_part = f"{BLUE}{cwd}{RESET}"
    model_part = f"{ORANGE}◆ {model_name}{RESET}"
    bar_part = f"{bar_color}{bar}{RESET}"
    tokens_part = f"{bar_color}{fmt_tokens(in_tok + out_tok)} (+{fmt_tokens(out_tok)}){RESET}"
    effort_part = f"{GREY}[{effort_level}]{RESET}" if effort_level else ""

    model_effort = f"{model_part} {effort_part}" if effort_part else model_part
    parts = [cwd_part, f"{GREY}|{RESET} {model_effort}"]
    if in_tok + out_tok > 0:
        parts.append(f"{GREY}|{RESET} {bar_part} {tokens_part}")
    if sess_total > 0:
        session_part = (
            f"{GREY}| ⧗ {fmt_duration(sess_secs)} "
            f"({fmt_tokens(sess_total)}, {fmt_tokens(sess_cached)} cached){RESET}"
        )
        parts.append(session_part)

    line1 = " ".join(parts)

    # Second line: unique skills, most-recent-first. Hidden (no newline) when empty.
    if skills:
        seen = set()
        unique = []
        for s in reversed(skills):
            if s not in seen:
                seen.add(s)
                unique.append(s)
        skills_line = f"{MAGENTA}skills: {', '.join(unique)}{RESET}"
        return f"{line1}\n{skills_line}"

    return line1


if __name__ == "__main__":
    print(get_status_line())

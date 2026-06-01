#!/usr/bin/env python3
import sys
import json
import os
import re
import time
import ssl
import subprocess
import urllib.request
import urllib.error
from datetime import datetime, timezone


def _ssl_context():
    """SSL context trusting the system store plus any corporate CA bundle.

    urllib ignores NODE_EXTRA_CA_CERTS (Node-only), so behind a TLS-intercepting
    proxy (e.g. Cato) the default store fails verification. Layer the corporate
    CA on top of the system store when present.
    """
    ctx = ssl.create_default_context()
    for var in ("NODE_EXTRA_CA_CERTS", "REQUESTS_CA_BUNDLE", "SSL_CERT_FILE"):
        path = os.environ.get(var)
        if path and os.path.isfile(path):
            try:
                ctx.load_verify_locations(path)
            except Exception:
                pass
    return ctx


CACHE_DIR = os.path.expanduser("~/.cache/claude-statusline")
USAGE_CACHE = os.path.join(CACHE_DIR, "usage.json")
USAGE_LOCK = os.path.join(CACHE_DIR, "usage.lock")
USAGE_BACKOFF = os.path.join(CACHE_DIR, "usage.backoff")
USAGE_TTL = (
    600  # seconds — refresh interval (usage data barely moves minute-to-minute)
)
USAGE_LOCK_TTL = 60  # seconds — minimum gap between refresh attempts
DEFAULT_BACKOFF = 900  # seconds — backoff when a failure carries no Retry-After


def _mtime(path):
    try:
        return os.path.getmtime(path)
    except OSError:
        return None


def read_usage_cache():
    """Return cached usage dict or None. No staleness check — caller decides."""
    try:
        with open(USAGE_CACHE) as f:
            return json.load(f)
    except Exception:
        return None


def _read_backoff_until():
    """Epoch until which refreshes should be suppressed, or 0 if none."""
    try:
        with open(USAGE_BACKOFF) as f:
            return float(f.read().strip())
    except Exception:
        return 0.0


def _write_backoff(seconds):
    try:
        os.makedirs(CACHE_DIR, exist_ok=True)
        with open(USAGE_BACKOFF, "w") as f:
            f.write(str(time.time() + seconds))
    except Exception:
        pass


def trigger_usage_refresh_if_stale():
    """Spawn a detached refresh if cache is older than TTL and no in-flight lock.

    Respects a backoff window written after failures (esp. HTTP 429 Retry-After)
    so transient errors don't turn every statusline render into a retry storm.
    """
    now = time.time()
    if now < _read_backoff_until():
        return
    cache_mt = _mtime(USAGE_CACHE)
    if cache_mt is not None and now - cache_mt < USAGE_TTL:
        return
    lock_mt = _mtime(USAGE_LOCK)
    if lock_mt is not None and now - lock_mt < USAGE_LOCK_TTL:
        return
    try:
        os.makedirs(CACHE_DIR, exist_ok=True)
        with open(USAGE_LOCK, "w") as f:
            f.write(str(now))
        subprocess.Popen(
            [sys.executable, os.path.realpath(__file__), "--refresh-usage"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
    except Exception:
        pass


def _read_oauth_token():
    path = os.path.expanduser("~/.claude/.credentials.json")
    try:
        with open(path) as f:
            data = json.load(f)
        return (data.get("claudeAiOauth") or {}).get("accessToken")
    except Exception:
        pass
    if sys.platform == "darwin":
        # Two entries can share service "Claude Code-credentials"; the live token
        # is under account "claude-code-user", while a stale per-user entry may
        # shadow it in a bare -s lookup. Prefer the named account, then fall back.
        for args in (
            ["-s", "Claude Code-credentials", "-a", "claude-code-user"],
            ["-s", "Claude Code-credentials"],
        ):
            try:
                out = subprocess.run(
                    ["security", "find-generic-password", *args, "-w"],
                    capture_output=True,
                    text=True,
                    timeout=5,
                )
                if out.returncode == 0:
                    data = json.loads(out.stdout.strip())
                    token = (data.get("claudeAiOauth") or {}).get("accessToken")
                    if token:
                        return token
            except Exception:
                pass
    return None


def refresh_usage_cache():
    """Fetch usage from Anthropic API and atomically write cache file."""
    try:
        token = _read_oauth_token()
        if not token:
            return
        req = urllib.request.Request(
            "https://api.anthropic.com/api/oauth/usage",
            headers={
                "Authorization": f"Bearer {token}",
                "anthropic-beta": "oauth-2025-04-20",
            },
        )
        with urllib.request.urlopen(
            req, timeout=5, context=_ssl_context()
        ) as resp:
            body = json.loads(resp.read())
        five = body.get("five_hour") or {}
        seven = body.get("seven_day") or {}
        out = {
            "fetched_at": int(time.time()),
            "session_pct": five.get("utilization"),
            "session_resets_at": five.get("resets_at"),
            "weekly_pct": seven.get("utilization"),
            "weekly_resets_at": seven.get("resets_at"),
        }
        os.makedirs(CACHE_DIR, exist_ok=True)
        tmp = USAGE_CACHE + ".tmp"
        with open(tmp, "w") as f:
            json.dump(out, f)
        os.replace(tmp, USAGE_CACHE)
        # Success: clear any prior backoff.
        try:
            os.remove(USAGE_BACKOFF)
        except OSError:
            pass
    except urllib.error.HTTPError as e:
        # Honor server-provided Retry-After (esp. 429); else back off a default.
        try:
            retry_after = int(e.headers.get("Retry-After") or 0)
        except (TypeError, ValueError):
            retry_after = 0
        _write_backoff(retry_after if retry_after > 0 else DEFAULT_BACKOFF)
    except Exception:
        # Network/SSL/parse errors: back off so renders don't retry-storm.
        _write_backoff(DEFAULT_BACKOFF)
    finally:
        try:
            os.remove(USAGE_LOCK)
        except OSError:
            pass


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


def find_project_root(start):
    """Walk up from `start` looking for a `.jj` or `.git` dir. Returns path or None."""
    if not start or not os.path.isdir(start):
        return None
    cur = os.path.abspath(start)
    while True:
        if os.path.isdir(os.path.join(cur, ".jj")) or os.path.isdir(
            os.path.join(cur, ".git")
        ):
            return cur
        parent = os.path.dirname(cur)
        if parent == cur:
            return None
        cur = parent


def fishlike_path(path):
    """Abbreviate all but the last path component to its leading char(s).

    Hidden dirs keep the leading dot (".config" -> ".c"). The last component
    is shown in full. "~" and "/" prefixes are preserved.
    """
    if not path:
        return path
    parts = path.split("/")
    last = len(parts) - 1
    out = []
    for i, p in enumerate(parts):
        if i == last or not p or p in ("~",):
            out.append(p)
            continue
        if p.startswith("."):
            out.append(p[:2])
        else:
            out.append(p[:1])
    return "/".join(out)


def get_status_line():
    try:
        data = json.load(sys.stdin)
    except Exception:
        return ""

    model_name = data.get("model", {}).get("display_name", "")
    model_name = re.sub(
        r"^(Haiku|Sonnet|Opus|Mythos) (.+)$",
        lambda m: m.group(1)[0] + m.group(2),
        model_name,
    )
    cw = data.get("context_window") or {}
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

    # ANSI Colors
    BLUE = "\033[34m"
    ORANGE = "\033[38;5;180m"
    GREY = "\033[38;5;244m"
    MAGENTA = "\033[35m"
    RED = "\033[31m"
    YELLOW = "\033[33m"
    RESET = "\033[0m"

    project_root = find_project_root(cwd)
    if project_root:
        project_name = os.path.basename(project_root) or project_root
        rel = os.path.relpath(cwd, project_root)
        if rel == ".":
            cwd_part = f"{BLUE}@{project_name}{RESET}"
        else:
            abbrev = fishlike_path(rel)
            cwd_part = f"{BLUE}@{project_name}{GREY}/{abbrev}{RESET}"
    else:
        home = os.path.expanduser("~")
        if cwd.startswith(home):
            cwd = cwd.replace(home, "~", 1)
        cwd_part = f"{BLUE}{fishlike_path(cwd)}{RESET}"

    # Color by input tokens in context
    ctx_color = GREY
    if in_tok >= 200_000:
        ctx_color = RED
    elif in_tok >= 100_000:
        ctx_color = ORANGE

    # Format token counts: no decimals for k, one decimal for M (e.g. 55k, 1.1M)
    def fmt_tokens(n):
        if n >= 1_000_000:
            return f"{n / 1_000_000:.1f}M"
        if n >= 1_000:
            return f"{round(n / 1_000)}k"
        return str(n)

    sess_in, sess_out, sess_cached, skills, sess_secs = parse_transcript(
        transcript_path
    )
    sess_total = sess_in + sess_out + sess_cached

    # Order: cwd | model | effort | bar | context tokens | session tokens
    model_part = f"{MAGENTA}{model_name}{RESET}"
    tokens_part = f"{ctx_color}{fmt_tokens(in_tok + out_tok)} (+{fmt_tokens(out_tok)}){RESET}"
    effort_part = effort_level

    def fmt_duration(secs):
        h, rem = divmod(secs, 3600)
        m = rem // 60
        if h and m:
            return f"{h}h{m:02d}m"
        if h:
            return f"{h}h"
        return f"{m}m"

    model_effort = f"{model_part} {effort_part}" if effort_part else model_part
    parts = [cwd_part, f"{GREY}|{RESET} {model_effort}"]
    if in_tok + out_tok > 0:
        parts.append(f"{GREY}|{RESET} {tokens_part}")

    def remaining_secs(iso_str):
        if not iso_str:
            return None
        try:
            t = datetime.fromisoformat(iso_str.replace("Z", "+00:00"))
            return max(0, int((t - datetime.now(timezone.utc)).total_seconds()))
        except Exception:
            return None

    def fmt_remaining(secs):
        days, rem = divmod(secs, 86400)
        hours, rem = divmod(rem, 3600)
        mins = rem // 60
        if days:
            return f"{days}d{hours}h" if hours else f"{days}d"
        if hours:
            return f"{hours}h{mins}m" if mins else f"{hours}h"
        return f"{mins}m"

    trigger_usage_refresh_if_stale()
    usage = read_usage_cache()
    if usage:
        bits = []
        for pct_key, reset_key in (
            ("session_pct", "session_resets_at"),
            ("weekly_pct", "weekly_resets_at"),
        ):
            pct = usage.get(pct_key)
            if not isinstance(pct, (int, float)):
                continue
            piece = f"{round(pct)}%"
            left = remaining_secs(usage.get(reset_key))
            if left is not None:
                piece += f" ({fmt_remaining(left)})"
            color = RED if pct >= 90 else YELLOW if pct >= 70 else GREY
            piece = f"{color}{piece}{GREY}"
            bits.append(piece)
        if bits:
            parts.append(f"{GREY}| {', '.join(bits)}{RESET}")

    if sess_total > 0:
        parts.append(
            f"{GREY}| ⧗ {fmt_duration(sess_secs)} "
            f"({fmt_tokens(sess_total)}, {fmt_tokens(sess_cached)} cached){RESET}"
        )

    line1 = " ".join(parts)

    # Second line: unique skills, most-recent-first. Hidden (no newline) when empty.
    if skills:
        seen = set()
        unique = []
        for s in reversed(skills):
            if s not in seen:
                seen.add(s)
                unique.append(s)
        skills_line = f"{GREY}skills: {', '.join(unique)}{RESET}"
        return f"{line1}\n{skills_line}"

    return line1


if __name__ == "__main__":
    if "--refresh-usage" in sys.argv:
        refresh_usage_cache()
        sys.exit(0)
    print(get_status_line())

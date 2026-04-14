#!/usr/bin/env python3
"""Race multiple LLM models and return the fastest successful response."""

import argparse
import asyncio
import bisect
import fcntl
import json
import math
import os
import random
import sys
import time
from collections.abc import Callable
from datetime import date, datetime
from pathlib import Path
from typing import Any, cast

DOTFILES_DIR: Path = Path(
    os.environ.get("DOTFILES_DIR", str(Path.home() / "dotfiles"))
)

# --- Configuration & Constants ---
STATS_DIR: Path = Path.home() / ".local/share/quick-prompt"
DAILY_STATS_DIR: Path = STATS_DIR / "daily"
AGGREGATED_STATS_FILE: Path = STATS_DIR / "aggregated_stats.json"
MODELS_CONFIG_FILE: Path = DOTFILES_DIR / "config/quick-prompt-models.json"


def get_model_config() -> list[dict[str, Any]]:
    """Load the model configuration from the configuration file."""
    config_file = MODELS_CONFIG_FILE if MODELS_CONFIG_FILE.exists() else None

    if not config_file:
        print(
            f"Error: Configuration file not found: {MODELS_CONFIG_FILE}",
            file=sys.stderr,
        )
        sys.exit(1)

    try:
        with open(config_file) as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        print(
            f"Error: Failed to load configuration from {config_file}: {e}",
            file=sys.stderr,
        )
        sys.exit(1)


def load_model_pool() -> list[str]:
    """Load the model pool from the configuration file."""
    config = get_model_config()
    for m in config:
        if "cli" not in m or "model_id" not in m:
            print(
                f"Error: Model '{m.get('model')}' is missing required fields"
                " 'cli' and/or 'model_id'.",
                file=sys.stderr,
            )
            sys.exit(1)
    pool = [m["model"] for m in config if m.get("enabled", True)]
    if not pool:
        print(
            "Error: No models are enabled in the configuration.",
            file=sys.stderr,
        )
        sys.exit(1)
    return pool


# Rolling window of results for invoking models. Used to determine which models
# to select for best latency.
STATS_ROLLING_WINDOW_SIZE = 100

_model_pool_cache: list[str] | None = None
_model_config_map_cache: dict[str, dict[str, Any]] | None = None


def _get_model_pool() -> list[str]:
    global _model_pool_cache
    if _model_pool_cache is None:
        _model_pool_cache = load_model_pool()
    return _model_pool_cache


def _get_model_config_map() -> dict[str, dict[str, Any]]:
    global _model_config_map_cache
    if _model_config_map_cache is None:
        _model_config_map_cache = {m["model"]: m for m in get_model_config()}
    return _model_config_map_cache


# How many samples of recent history to display in the stats table.
STATS_HISTORY_COUNT = 30

# ANSI color codes used in the stats table.
_STATS_RESET = "\033[0m"
_STATS_GRAY = "\033[90m"
_STATS_MODEL_COLOR = "\033[36m"
# Gradient endpoints: 0th percentile = warm green, 100th = warm yellow
_G0 = (131, 206, 56)
_G100 = (255, 210, 124)

# How many samples to use when calculating model weight.
MODEL_WEIGHT_SAMPLE_COUNT = 10

# How many models to race by default.
DEFAULT_MODEL_COUNT = 3

# Timeout for detached model runs
TIMEOUT_DURATION = 60

# Global state for verbosity
_verbose: bool = False


# --- Helper Functions ---


def debug(msg: str) -> None:
    """Print a debug message to stderr if VERBOSE is enabled."""
    if _verbose:
        # Gray color using ANSI escape codes for subtle output
        print(f"\033[90m[debug] {msg}\033[0m", file=sys.stderr)


def get_histogram_bucket(duration: float) -> str:
    """
    Calculate an exponential histogram bucket for a given duration.

    Buckets follow the formula: 0.1 * (1.1^n)
    """
    if duration < 0.1:
        return "0.0-0.1"
    if duration >= 60.0:
        return "60.0+"

    # Exponential buckets: 0.1 * (1.1^n)
    # n = log(duration/0.1) / log(1.1)
    n = math.floor(math.log(duration / 0.1) / math.log(1.1))
    lower = 0.1 * (1.1**n)
    upper = 0.1 * (1.1 ** (n + 1))
    return f"{lower:.2f}-{upper:.2f}"


# --- Stats & Telemetry ---


def _update_json_file(
    path: Path,
    default: Any,  # noqa: ANN401
    fn: Callable[[Any], None],
) -> None:
    """
    Lock path exclusively and update JSON data.

    Load JSON (or use default on decode error), call fn(data), write back.
    """
    if not path.exists():
        path.write_text(json.dumps(default))
    with open(path, "r+") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        try:
            try:
                data = json.load(f)
            except json.JSONDecodeError:
                data = default
            fn(data)
            f.seek(0)
            f.truncate()
            json.dump(data, f, indent=2)
            f.flush()
            os.fsync(f.fileno())
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)


def update_aggregated_stats(
    model: str,
    duration: float,
    start_timestamp: str,
    end_timestamp: str,
    error: str | None = None,
) -> None:
    """Update the global aggregated statistics file for a single model run."""
    AGGREGATED_STATS_FILE.parent.mkdir(parents=True, exist_ok=True)

    def _update(data: dict[str, Any]) -> None:
        stats = data.setdefault(
            model,
            {
                "runs": 0,
                "wins": 0,
                "fails": 0,
                # FIFO: [{start, end, duration, error}, ...]
                "last_run_results": [],
                "avg": 0,
                "min": None,
                "max": 0,
                "histogram": {},
            },
        )

        entry = {
            "start_timestamp": start_timestamp,
            "end_timestamp": end_timestamp,
            "duration": duration,
            "error": error,
        }

        stats["runs"] += 1
        stats["last_run_results"].append(entry)

        if len(stats["last_run_results"]) > STATS_ROLLING_WINDOW_SIZE:
            stats["last_run_results"].pop(0)

        # Recalculate rolling average from window
        win_durations = [
            r["duration"]
            for r in stats["last_run_results"]
            if r["error"] is None
        ]
        if win_durations:
            stats["avg"] = round(sum(win_durations) / len(win_durations), 3)
        else:
            stats["avg"] = 0

        if error is None:
            stats["wins"] += 1
            d = duration

            # Update historical min/max
            if stats["min"] is None or d < stats["min"]:
                stats["min"] = d
            stats["max"] = max(stats["max"], d)

            # Update Histogram (historical/all-time)
            bucket = get_histogram_bucket(d)
            stats["histogram"][bucket] = stats["histogram"].get(bucket, 0) + 1
        else:
            stats["fails"] += 1

    _update_json_file(AGGREGATED_STATS_FILE, {}, _update)


def log_stats(
    model: str,
    duration: float,
    start_timestamp: str,
    end_timestamp: str,
    error: str | None = None,
) -> None:
    """
    Log individual model results to a daily JSON file and update global stats.

    Uses file locking to prevent corruption from concurrent worker writes.
    """
    DAILY_STATS_DIR.mkdir(parents=True, exist_ok=True)
    today = date.today().isoformat()
    daily_file = DAILY_STATS_DIR / f"{today}.json"

    entry = {
        "start_timestamp": start_timestamp,
        "end_timestamp": end_timestamp,
        "model": model,
        "duration": duration,
        "error": error,
    }

    try:
        _update_json_file(daily_file, [], lambda data: data.append(entry))
    except Exception as e:
        debug(f"Failed to log to daily stats: {e}")

    # Update global aggregates with robust locking
    update_aggregated_stats(
        model, duration, start_timestamp, end_timestamp, error
    )


def _compute_global_distributions(
    data: dict[str, Any],
) -> tuple[list[float], list[float]]:
    """Return (sorted all_durations, sorted all_averages) across all models."""
    all_durations: list[float] = []
    all_averages: list[float] = []
    for s in data.values():
        results: list[dict[str, Any]] = s.get("last_run_results", [])
        win_durations: list[float] = [
            r["duration"] for r in results if r.get("error") is None
        ]
        if win_durations:
            all_averages.append(sum(win_durations) / len(win_durations))
            all_durations.extend(win_durations)
    all_durations.sort()
    all_averages.sort()
    return all_durations, all_averages


def _format_ts(raw: Any) -> str:  # noqa: ANN401
    """Format an ISO timestamp to 'YYYY-MM-DD HH:MM:SS' (19 chars)."""
    if not raw:
        return " " * 19
    try:
        dt = datetime.fromisoformat(raw)
        return dt.strftime("%Y-%m-%d %H:%M:%S")
    except (ValueError, TypeError):
        return str(raw)[:19].replace("T", " ").ljust(19)


def _render_history_cell(
    results: list[dict[str, Any]],
    all_durations: list[float],
) -> str:
    """Render the timestamped history-graph cell for one model table row."""
    gray = _STATS_GRAY
    reset = _STATS_RESET
    G0_R, G0_G, G0_B = _G0
    G100_R, G100_G, G100_B = _G100
    global_n = len(all_durations)
    num_symbols = 0
    first_ts = " " * 19
    last_ts = " " * 19
    history_graph = ""

    if results:
        recent = results[-STATS_HISTORY_COUNT:]
        num_symbols = len(recent)
        first_ts = _format_ts(
            recent[0].get("end_timestamp") or recent[0].get("start_timestamp")
        )
        last_ts = _format_ts(
            recent[-1].get("end_timestamp") or recent[-1].get("start_timestamp")
        )

        if global_n > 0:
            symbols = ["▂", "▃", "▄", "▅", "▆", "▇", "█"]
            for res in recent:
                if res.get("error") is not None:
                    history_graph += "\033[38;2;222;112;112m█\033[0m"
                else:
                    d = res["duration"]
                    rank = bisect.bisect_right(all_durations, d)
                    percentile = (
                        ((rank - 1) / (global_n - 1)) * 100
                        if global_n > 1
                        else 0
                    )
                    bucket = min(int(percentile // (100 / 7)), 6)
                    ratio = percentile / 100
                    r = int(G0_R + (G100_R - G0_R) * ratio)
                    g = int(G0_G + (G100_G - G0_G) * ratio)
                    b = int(G0_B + (G100_B - G0_B) * ratio)
                    color = f"\033[38;2;{r};{g};{b}m"
                    history_graph += f"{color}{symbols[bucket]}\033[0m"
        else:
            for res in recent:
                if res.get("error") is not None:
                    history_graph += "\033[38;2;255;0;0mx\033[0m"

    padding = " " * (STATS_HISTORY_COUNT - num_symbols)
    return (
        f"{gray}{first_ts}{reset} {history_graph}"
        f"{padding} {gray}{last_ts}{reset}"
    )


def _avg_color(avg: float, all_averages: list[float]) -> str:
    """Return an ANSI RGB color code for avg relative to all_averages."""
    avg_n = len(all_averages)
    if avg <= 0 or avg_n == 0:
        return _STATS_GRAY
    G0_R, G0_G, G0_B = _G0
    G100_R, G100_G, G100_B = _G100
    avg_rank = bisect.bisect_right(all_averages, avg)
    avg_percentile = ((avg_rank - 1) / (avg_n - 1)) * 100 if avg_n > 1 else 0
    ratio = avg_percentile / 100
    r = int(G0_R + (G100_R - G0_R) * ratio)
    g = int(G0_G + (G100_G - G0_G) * ratio)
    b = int(G0_B + (G100_B - G0_B) * ratio)
    return f"\033[38;2;{r};{g};{b}m"


def _ensure_all_models_in_data(
    data: dict[str, Any], model_config: list[dict[str, Any]]
) -> None:
    """Ensure each configured model has an entry in data."""
    for m_info in model_config:
        model_name = m_info["model"]
        if model_name not in data:
            data[model_name] = {
                "runs": 0,
                "wins": 0,
                "fails": 0,
                "last_run_results": [],
                "avg": 0,
                "min": None,
                "max": 0,
                "histogram": {},
            }


def print_stats() -> None:
    """Print aggregated statistics for all models in a pretty table."""
    if not AGGREGATED_STATS_FILE.exists():
        print("No statistics found.")
        return

    with open(AGGREGATED_STATS_FILE) as f:
        try:
            # Shared lock for reading
            fcntl.flock(f, fcntl.LOCK_SH)
            data: dict[str, Any] = json.load(f)
        except (OSError, json.JSONDecodeError):
            print("Error: Could not read statistics.")
            return
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)

    if not data:
        print("No statistics recorded yet.")
        return

    # Get all configured models to ensure they are represented in the table
    model_config = get_model_config()
    enabled_models = {
        m["model"] for m in model_config if m.get("enabled", True)
    }

    # Ensure all configured models are in data, even if they haven't run yet
    _ensure_all_models_in_data(data, model_config)

    def _window_avg(stats: dict[str, Any]) -> float:
        win_durations = [
            r["duration"]
            for r in stats.get("last_run_results", [])
            if r.get("error") is None
        ]
        return sum(win_durations) / len(win_durations) if win_durations else 1e9

    sorted_items = sorted(data.items(), key=lambda item: _window_avg(item[1]))

    gray = _STATS_GRAY
    reset = _STATS_RESET
    model_color = _STATS_MODEL_COLOR

    # Calculate column widths
    model_names = [m["model"] for m in model_config]
    any_disabled = any(not m.get("enabled", True) for m in model_config)
    model_col_width = max(
        max(len(name) for name in model_names) + (11 if any_disabled else 0),
        len("Model"),
    )
    avg_col_width = 7  # Sufficient for "12.345s"
    history_col_width = 19 + 1 + STATS_HISTORY_COUNT + 1 + 19

    model_sep_width = model_col_width + 2
    avg_sep_width = avg_col_width + 2
    history_sep_width = history_col_width + 2

    # Header
    print(
        f"{gray}┌"
        + "─" * model_sep_width
        + "┬"
        + "─" * avg_sep_width
        + "┬"
        + "─" * history_sep_width
        + f"┐{reset}"
    )
    print(
        f"{gray}│{reset} \033[1m{'Model':<{model_col_width}}\033[0m"
        f" {gray}│{reset} "
        f"\033[1m{'Avg':>{avg_col_width}}\033[0m {gray}│{reset} "
        f"\033[1m{'History':<{history_col_width}}\033[0m {gray}│{reset}"
    )
    print(
        f"{gray}├"
        + "─" * model_sep_width
        + "┼"
        + "─" * avg_sep_width
        + "┼"
        + "─" * history_sep_width
        + f"┤{reset}"
    )

    all_durations, all_averages = _compute_global_distributions(data)

    for i, (model, stats) in enumerate(sorted_items):
        win_durations = [
            r["duration"]
            for r in stats.get("last_run_results", [])
            if r.get("error") is None
        ]
        wins_in_window = len(win_durations)
        avg = sum(win_durations) / wins_in_window if wins_in_window > 0 else 0
        avg_str = f"{avg:.3f}s" if wins_in_window > 0 else "N/A"
        color = _avg_color(avg, all_averages) if wins_in_window > 0 else gray

        is_enabled = model in enabled_models
        display_name = model if is_enabled else f"{model} (disabled)"
        name_color = model_color if is_enabled else gray

        combined_history = _render_history_cell(
            stats.get("last_run_results", []), all_durations
        )

        print(
            f"{gray}│{reset} {name_color}"
            f"{display_name:<{model_col_width}}{reset}"
            f" {gray}│{reset} "
            f"{color}{avg_str:>{avg_col_width}}{reset} {gray}│{reset} "
            f"{combined_history} {gray}│{reset}"
        )

        if i < len(sorted_items) - 1:
            print(
                f"{gray}├"
                + "─" * model_sep_width
                + "┼"
                + "─" * avg_sep_width
                + "┼"
                + "─" * history_sep_width
                + f"┤{reset}"
            )
        else:
            print(
                f"{gray}└"
                + "─" * model_sep_width
                + "┴"
                + "─" * avg_sep_width
                + "┴"
                + "─" * history_sep_width
                + f"┘{reset}"
            )


# --- Core Logic ---


def _parse_ndjson_output(stdout: bytes) -> tuple[str, str | None]:
    """Parse NDJSON lines from opencode; return (text, error_msg)."""
    res_text = ""
    error_msg = None
    for line in stdout.decode().splitlines():
        try:
            res_json = json.loads(line)
            if res_json.get("type") == "text":
                res_text += res_json.get("part", {}).get("text", "")
            elif res_json.get("type") == "error":
                error_msg = res_json.get("error", {}).get("data", {}).get(
                    "message"
                ) or res_json.get("error", {}).get("name")
        except json.JSONDecodeError:
            continue
    return res_text, error_msg


async def run_model(model: str, prompt: str) -> dict[str, Any]:
    """
    Invoke the appropriate CLI for a specific model and return the result.

    Dispatches to opencode or claude CLI based on model config.
    Parses NDJSON output for opencode; plain text for claude.
    """
    debug(f"Starting model: {model}")
    start_time = time.perf_counter()
    start_timestamp = datetime.now().isoformat()

    model_info = _get_model_config_map().get(model, {})
    cli: str = model_info["cli"]
    model_id: str = model_info["model_id"]

    if cli == "claude":
        cmd = [
            "claude",
            "-p",
            prompt,
            "--model",
            model_id,
            "--max-turns",
            "1",
        ]
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdin=asyncio.subprocess.DEVNULL,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
    else:
        env = os.environ.copy()
        env["OPENCODE_DISABLE_DEFAULT_PLUGINS"] = "true"
        env["OPENCODE_DISABLE_CLAUDE_CODE"] = "true"
        cmd = [
            "opencode",
            "run",
            "--model",
            model_id,
            "--variant",
            "low",
            "--format",
            "json",
        ]
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=env,
        )

    communicate_input = None if cli == "claude" else prompt.encode()

    try:
        stdout, stderr = await asyncio.wait_for(
            proc.communicate(input=communicate_input), timeout=TIMEOUT_DURATION
        )
        duration = time.perf_counter() - start_time
        end_timestamp = datetime.now().isoformat()
        debug(
            f"Model {model} finished in {duration:.3f}s"
            f" with exit code {proc.returncode}"
        )

        if proc.returncode != 0:
            err_msg = stderr.decode().strip()
            debug(f"Model {model} failed with stderr: {err_msg}")
            return {
                "error": err_msg,
                "model": model,
                "code": proc.returncode,
                "success": False,
                "duration": round(duration, 3),
                "start_timestamp": start_timestamp,
                "end_timestamp": end_timestamp,
            }

        if cli == "claude":
            res_text = stdout.decode().strip()
            error_msg = None
        else:
            res_text, error_msg = _parse_ndjson_output(stdout)

        if error_msg and not res_text:
            debug(f"Model {model} returned logic error: {error_msg}")
            return {
                "error": error_msg,
                "model": model,
                "success": False,
                "duration": round(duration, 3),
                "start_timestamp": start_timestamp,
                "end_timestamp": end_timestamp,
            }

        debug(f"Model {model} successfully produced {len(res_text)} chars")
        return {
            "text": res_text,
            "model": model,
            "duration": round(duration, 3),
            "start_time": start_time,
            "success": True,
            "start_timestamp": start_timestamp,
            "end_timestamp": end_timestamp,
        }
    except TimeoutError:
        end_timestamp = datetime.now().isoformat()
        debug(f"Model {model} timed out after {TIMEOUT_DURATION}s")
        if proc.returncode is None:
            try:
                proc.kill()
            except ProcessLookupError:
                pass
        return {
            "error": "Timeout",
            "model": model,
            "success": False,
            "duration": TIMEOUT_DURATION,
            "start_timestamp": start_timestamp,
            "end_timestamp": end_timestamp,
        }
    except Exception as e:
        end_timestamp = datetime.now().isoformat()
        debug(f"Model {model} raised exception: {e}")
        if proc.returncode is None:
            try:
                proc.kill()
            except ProcessLookupError:
                pass
        return {
            "error": str(e),
            "model": model,
            "success": False,
            "duration": round(time.perf_counter() - start_time, 3),
            "start_timestamp": start_timestamp,
            "end_timestamp": end_timestamp,
        }


async def run_worker(model: str, prompt: str) -> None:
    """
    Entry point for internal worker process.

    Run model, update stats, and output result if successful.
    """
    try:
        result = await run_model(model, prompt)

        # Log results independently (even if we lost the race in the parent)
        log_stats(
            model,
            result["duration"],
            result["start_timestamp"],
            result["end_timestamp"],
            result.get("error"),
        )

        if result["success"]:
            # Print text for parent to capture. Use try/except to ignore
            # BrokenPipeError if the parent has already exited.
            try:
                print(result["text"])
                sys.stdout.flush()
            except BrokenPipeError:
                pass
    except Exception as e:
        debug(f"Worker for {model} failed: {e}")


def get_model_weights() -> dict[str, float]:
    """
    Calculate weights for each model in MODEL_POOL based on historical stats.

    Weight = 1 / (avg_duration^2). Models without stats get a default weight.
    """
    stats_data: dict[str, Any] = {}
    if AGGREGATED_STATS_FILE.exists():
        with open(AGGREGATED_STATS_FILE) as f:
            try:
                # Shared lock
                fcntl.flock(f, fcntl.LOCK_SH)
                stats_data = cast(dict[str, Any], json.load(f))
            except (OSError, json.JSONDecodeError):
                pass
            finally:
                fcntl.flock(f, fcntl.LOCK_UN)

    weights: dict[str, float] = {}
    # Use existing averages to determine a baseline for new models
    known_averages: list[float] = [
        s["avg"] for s in stats_data.values() if s.get("wins", 0) > 0
    ]
    # Default to 4.5s if no stats exist, otherwise use the average of averages
    default_duration = (
        sum(known_averages) / len(known_averages) if known_averages else 4.5
    )

    # Cap the default duration to keep it competitive but not dominant
    default_duration = max(3.0, min(7.0, default_duration))

    for model in _get_model_pool():
        stats = stats_data.get(model, {})
        avg = (
            stats.get("avg", default_duration)
            if any(
                r.get("error") is None
                for r in stats.get("last_run_results", [])
            )
            else default_duration
        )

        # Weight follows inverse square law: 2x faster = 4x more likely
        weight = 1.0 / (max(0.1, avg) ** 2)

        # Penalize recent failures (last N samples)
        # If the last N samples for a model are failures, heavily penalize it.
        results = stats.get("last_run_results", [])
        if results:
            recent_outcomes = [
                r.get("error") is None
                for r in results[-MODEL_WEIGHT_SAMPLE_COUNT:]
            ]
            if len(recent_outcomes) >= MODEL_WEIGHT_SAMPLE_COUNT and not any(
                recent_outcomes
            ):
                # All of the last N failed! Heavy penalty.
                weight *= 0.01
            elif any(o is False for o in recent_outcomes):
                # At least one failure in the recent window.
                fail_rate = recent_outcomes.count(False) / len(recent_outcomes)
                weight *= 1.0 - (0.5 * fail_rate)

        weights[model] = weight

    return weights


def _select_models() -> list[str]:
    """Select models for a race via env override or weighted sampling."""
    override = os.getenv("QUICK_PROMPT_MODEL")
    if override:
        return [override]

    weights_dict = get_model_weights()
    models = list(weights_dict.keys())
    weights = [weights_dict[m] for m in models]

    if _verbose:
        debug("Model weights for selection:")
        for m, w in weights_dict.items():
            debug(f"  {m}: {w:.4f}")

    # Weighted sampling without replacement
    selected: list[str] = []
    target_count = min(len(_get_model_pool()), DEFAULT_MODEL_COUNT)
    temp_models = models[:]
    temp_weights = weights[:]
    while len(selected) < target_count and temp_models:
        choice = random.choices(temp_models, weights=temp_weights, k=1)[0]
        selected.append(choice)
        idx = temp_models.index(choice)
        temp_models.pop(idx)
        temp_weights.pop(idx)
    return selected


async def _await_first_winner(
    worker_tasks: list[asyncio.Task[bytes]],
    worker_procs: list[asyncio.subprocess.Process],
) -> tuple[str | None, str | None]:
    """Wait for the first task to return non-empty output; clean up the rest."""
    winner = None
    winner_model = None
    try:
        while worker_tasks:
            done, _ = await asyncio.wait(
                worker_tasks, return_when=asyncio.FIRST_COMPLETED
            )
            for task in done:
                res_text: str = task.result().decode().strip()
                model: str = task.get_name()
                if res_text:
                    debug(f"Model {model} won the race.")
                    winner = res_text
                    winner_model = model
                    break
                else:
                    debug(f"Model {model} failed or returned no text.")
                    worker_tasks.remove(task)
            if winner:
                break
    finally:
        remaining: list[asyncio.Task[bytes]] = [
            t for t in worker_tasks if not t.done()
        ]
        for t in remaining:
            t.cancel()
        if remaining:
            await asyncio.gather(*remaining, return_exceptions=True)
        for proc in worker_procs:
            try:
                transport = getattr(proc, "_transport", None)
                if transport is not None:
                    transport.close()
            except Exception:
                pass
        await asyncio.sleep(0)  # let close callbacks run before loop shuts down
    return winner, winner_model


async def race_models(prompt: str) -> tuple[str, str]:
    """
    Race multiple models in parallel using detached internal worker processes.

    The first one to return a successful response wins.
    Returns (winning response text, winning model name), or raises RuntimeError
    if all fail.
    """
    selected_models = _select_models()
    debug(f"Selected models for race: {selected_models}")

    # 2. Start workers as detached processes
    # We use start_new_session=True so they outlive the parent script.
    # They communicate their result via stdout.
    worker_procs: list[asyncio.subprocess.Process] = []
    worker_tasks: list[asyncio.Task[bytes]] = []
    for model in selected_models:
        cmd = [sys.executable, __file__, "--internal-worker", model]
        if _verbose:
            cmd.append("--verbose")

        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.DEVNULL,
            start_new_session=True,
        )
        assert proc.stdin is not None
        assert proc.stdout is not None
        # Send prompt immediately
        proc.stdin.write(prompt.encode())
        await proc.stdin.drain()
        proc.stdin.close()

        # Capture output from worker
        worker_procs.append(proc)
        worker_tasks.append(asyncio.create_task(proc.stdout.read(), name=model))

    # 3. Wait for the first worker to produce output
    # Cancel remaining read tasks and close subprocess transports explicitly
    # while the event loop is still running. Without this, transports are
    # closed by GC after the loop shuts down, causing "Event loop is closed"
    # errors from BaseSubprocessTransport.__del__.
    winner, winner_model = await _await_first_winner(worker_tasks, worker_procs)
    if winner is None or winner_model is None:
        raise RuntimeError("All models failed to produce a response.")
    return winner, winner_model


async def verify_models() -> None:
    """Check the health and latency of all models in the pool."""
    print(f"Verifying {len(_get_model_pool())} models...")
    tasks = [run_model(m, "hi") for m in _get_model_pool()]
    results = await asyncio.gather(*tasks)

    all_ok = True
    for model, res in zip(_get_model_pool(), results, strict=False):
        if res.get("success"):
            print(f"\033[92m[OK]\033[0m {model} ({res['duration']}s)")
        else:
            all_ok = False
            err = res.get("error", "Unknown error")
            # Truncate long error messages for cleaner output
            err = (err[:75] + "...") if len(err) > 75 else err
            print(f"\033[91m[FAIL]\033[0m {model}: {err}")

    if not all_ok:
        sys.exit(1)


# --- Entry Point ---


def run(prompt: str) -> tuple[str, str]:
    """
    Run the model race and return (winning response text, winning model name).

    Raises RuntimeError on failure.
    """
    return asyncio.run(race_models(prompt))


def main() -> None:
    """Parse arguments and run the model race or a stats/ping subcommand."""
    parser = argparse.ArgumentParser(
        description="Race LLM models for the fastest response."
    )
    parser.add_argument(
        "prompt", nargs="*", help="The prompt to send to the models."
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Enable verbose debug logging.",
    )
    parser.add_argument(
        "--ping-models",
        action="store_true",
        help="Ping all models in the pool to verify functionality.",
    )
    parser.add_argument(
        "--stats",
        action="store_true",
        help="Print aggregated model statistics.",
    )
    parser.add_argument(
        "--internal-worker",
        help=argparse.SUPPRESS,  # Hidden internal flag
    )

    args = parser.parse_args()

    global _verbose
    _verbose = args.verbose

    if args.internal_worker:
        # Run as a worker. Prompt is provided via stdin.
        prompt = sys.stdin.read().strip()
        asyncio.run(run_worker(args.internal_worker, prompt))
        sys.exit(0)

    if args.ping_models:
        asyncio.run(verify_models())
        sys.exit(0)

    if args.stats:
        print_stats()
        sys.exit(0)

    if args.prompt:
        user_prompt = " ".join(args.prompt)
    elif not sys.stdin.isatty():
        user_prompt = sys.stdin.read().strip()
    else:
        parser.print_help()
        sys.exit(1)

    if not user_prompt:
        print("Error: No prompt provided.", file=sys.stderr)
        sys.exit(1)

    debug("Script initialized, starting race...")
    try:
        result, winner_model = asyncio.run(race_models(user_prompt))
    except Exception as e:
        debug(f"Race failed: {e}")
        print(f"Race failed: {e}", file=sys.stderr)
        os._exit(1)
    print(f"\033[90m({winner_model})\033[0m", file=sys.stderr)
    print(result)
    sys.stdout.flush()
    sys.stderr.flush()
    os._exit(0)


if __name__ == "__main__":
    main()

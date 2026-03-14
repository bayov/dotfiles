#!/usr/bin/env python3
import argparse
import asyncio
import fcntl
import json
import math
import os
import random
import sys
import time
from datetime import date, datetime
from pathlib import Path
from typing import Any, Dict, List, Optional, Set

# --- Configuration & Constants ---
STATS_DIR: Path = Path.home() / ".local/share/quick_prompt"
DAILY_STATS_DIR: Path = STATS_DIR / "daily"
AGGREGATED_STATS_FILE: Path = STATS_DIR / "aggregated_stats.json"

MODEL_POOL: List[str] = [
    "opencode/big-pickle",
    "opencode/gpt-5-nano",
    "opencode/mimo-v2-flash-free",
    "opencode/minimax-m2.5-free",
    "opencode/nemotron-3-super-free",
]

# Global state for verbosity
VERBOSE: bool = False


# --- Helper Functions ---


def debug(msg: str) -> None:
    """Prints a debug message to stderr if VERBOSE is enabled."""
    if VERBOSE:
        # Gray color using ANSI escape codes for subtle output
        print(f"\033[90m[debug] {msg}\033[0m", file=sys.stderr)


def get_histogram_bucket(duration: float) -> str:
    """
    Calculates an exponential histogram bucket for a given duration.
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


def update_aggregated_stats(
    selected_models: List[str],
    winner: Optional[str],
    duration: float,
    failed_models: Set[str],
) -> None:
    """
    Updates the global aggregated statistics file with thread/process safety via file locking.
    Tracks wins, fails, and duration distributions for each model.
    """
    AGGREGATED_STATS_FILE.parent.mkdir(parents=True, exist_ok=True)

    # Ensure the file exists before attempting to lock it
    if not AGGREGATED_STATS_FILE.exists():
        with open(AGGREGATED_STATS_FILE, "w") as f:
            json.dump({}, f)

    with open(AGGREGATED_STATS_FILE, "r+") as f:
        try:
            # Exclusive lock to prevent concurrent write issues
            fcntl.flock(f, fcntl.LOCK_EX)
            try:
                data: Dict[str, Any] = json.load(f)
            except json.JSONDecodeError:
                data = {}

            # Update stats for every model involved in the race
            for m in selected_models:
                stats = data.setdefault(
                    m,
                    {
                        "runs": 0,
                        "wins": 0,
                        "fails": 0,
                        "durations": [],
                        "avg": 0,
                        "min": None,
                        "max": 0,
                        "median": 0,
                        "histogram": {},
                    },
                )

                stats["runs"] += 1
                if m == winner:
                    stats["wins"] += 1
                    d = duration
                    stats["durations"].append(d)
                    stats["durations"].sort()

                    # Update min/max/avg
                    if stats["min"] is None or d < stats["min"]:
                        stats["min"] = d
                    stats["max"] = max(stats["max"], d)
                    stats["avg"] = sum(stats["durations"]) / len(
                        stats["durations"]
                    )

                    # Calculate Median
                    mid = len(stats["durations"]) // 2
                    if len(stats["durations"]) % 2 == 0:
                        stats["median"] = round(
                            (
                                stats["durations"][mid - 1]
                                + stats["durations"][mid]
                            )
                            / 2,
                            3,
                        )
                    else:
                        stats["median"] = round(stats["durations"][mid], 3)

                    stats["avg"] = round(stats["avg"], 3)

                    # Update Histogram
                    bucket = get_histogram_bucket(d)
                    stats["histogram"][bucket] = (
                        stats["histogram"].get(bucket, 0) + 1
                    )

                if m in failed_models:
                    stats["fails"] += 1

            # Save updated data back to file
            f.seek(0)
            f.truncate()
            json.dump(data, f, indent=2)
            f.flush()
            os.fsync(f.fileno())
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)


def log_stats(
    selected_models: List[str],
    winner: Optional[str],
    duration: float,
    failed_models: Set[str],
) -> None:
    """
    Logs individual race results to a daily JSON file and updates global aggregates.
    """
    DAILY_STATS_DIR.mkdir(parents=True, exist_ok=True)
    today = date.today().isoformat()
    daily_file = DAILY_STATS_DIR / f"{today}.json"

    entry = {
        "timestamp": datetime.now().isoformat(),
        "models": selected_models,
        "winner": winner,
        "duration": duration,
        "failures": list(failed_models),
    }

    # Log to daily file (load, append, save)
    # Note: Daily logs don't use locking as they are less critical and
    # usually sequential in normal usage, but it could be added if needed.
    stats = []
    if daily_file.exists():
        with open(daily_file, "r") as f:
            try:
                stats = json.load(f)
            except json.JSONDecodeError:
                stats = []

    stats.append(entry)
    with open(daily_file, "w") as f:
        json.dump(stats, f, indent=2)

    # Update global aggregates with robust locking
    update_aggregated_stats(selected_models, winner, duration, failed_models)


def print_stats() -> None:
    """
    Prints aggregated statistics for all models in a pretty, colorized table.
    """
    if not AGGREGATED_STATS_FILE.exists():
        print("No statistics found.")
        return

    with open(AGGREGATED_STATS_FILE, "r") as f:
        try:
            # Shared lock for reading
            fcntl.flock(f, fcntl.LOCK_SH)
            data: Dict[str, Any] = json.load(f)
        except (json.JSONDecodeError, IOError):
            print("Error: Could not read statistics.")
            return
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)

    if not data:
        print("No statistics recorded yet.")
        return

    # Sort models by average duration (fastest first)
    # Models with no wins are placed at the end
    sorted_items = sorted(
        data.items(),
        key=lambda item: item[1]["avg"] if item[1]["wins"] > 0 else 1e9,
    )

    # Header
    print(
        f"\033[1m{'Model':<35} | {'Avg':>7} | {'Med':>7} | {'Win Rate':>10} | {'Fails':>5} | {'Min/Max':>13}\033[0m"
    )
    print("-" * 92)

    for model, stats in sorted_items:
        runs = stats.get("runs", 0)
        wins = stats.get("wins", 0)
        fails = stats.get("fails", 0)
        avg = stats.get("avg", 0)
        median = stats.get("median", 0)
        min_val = stats.get("min", 0)
        max_val = stats.get("max", 0)

        win_rate = f"{wins}/{runs}"

        # Formatting values
        avg_str = f"{avg:.3f}s" if wins > 0 else "N/A"
        med_str = f"{median:.3f}s" if wins > 0 else "N/A"
        min_max_str = f"{min_val:.2f}s/{max_val:.2f}s" if wins > 0 else "N/A"

        # Colors
        model_color = "\033[36m"  # Cyan
        avg_color = "\033[32m" if wins > 0 else "\033[90m"  # Green / Gray
        fail_color = "\033[31m" if fails > 0 else ""  # Red if > 0
        reset = "\033[0m"

        print(
            f"{model_color}{model:<35}{reset} | "
            f"{avg_color}{avg_str:>7}{reset} | "
            f"{med_str:>7} | "
            f"{win_rate:>10} | "
            f"{fail_color}{fails:>5}{reset} | "
            f"{min_max_str:>13}"
        )


# --- Core Logic ---


async def run_model(model: str, prompt: str) -> Dict[str, Any]:
    """
    Invokes the opencode CLI for a specific model and returns the result.
    Parses NDJSON output and handles errors/timeouts.
    """
    debug(f"Starting model: {model}")
    start_time = time.perf_counter()

    # Environment setup to minimize overhead and disable interactive features
    env = os.environ.copy()
    env["OPENCODE_DISABLE_DEFAULT_PLUGINS"] = "true"
    env["OPENCODE_DISABLE_CLAUDE_CODE"] = "true"

    # opencode run --model <model> --variant low --format json
    cmd = [
        "opencode",
        "run",
        "--model",
        model,
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

    try:
        # Send prompt and wait for result
        stdout, stderr = await proc.communicate(input=prompt.encode())
        duration = time.perf_counter() - start_time
        debug(
            f"Model {model} finished in {duration:.3f}s with exit code {proc.returncode}"
        )

        if proc.returncode != 0:
            err_msg = stderr.decode().strip()
            debug(f"Model {model} failed with stderr: {err_msg}")
            return {
                "error": err_msg,
                "model": model,
                "code": proc.returncode,
                "success": False,
            }

        # Parse NDJSON output (multiple JSON objects, one per line)
        res_text = ""
        error_msg = None
        for line in stdout.decode().splitlines():
            try:
                res_json = json.loads(line)
                if res_json.get("type") == "text":
                    res_text += res_json.get("part", {}).get("text", "")
                elif res_json.get("type") == "error":
                    # Extract descriptive error message from opencode JSON
                    error_msg = res_json.get("error", {}).get("data", {}).get(
                        "message"
                    ) or res_json.get("error", {}).get("name")
            except json.JSONDecodeError:
                continue

        if error_msg and not res_text:
            debug(f"Model {model} returned logic error: {error_msg}")
            return {"error": error_msg, "model": model, "success": False}

        debug(f"Model {model} successfully produced {len(res_text)} chars")
        return {
            "text": res_text,
            "model": model,
            "duration": round(duration, 3),
            "start_time": start_time,
            "success": True,
        }
    except asyncio.CancelledError:
        debug(f"Model {model} was cancelled")
        if proc.returncode is None:
            try:
                proc.kill()
            except ProcessLookupError:
                pass
        raise
    except Exception as e:
        debug(f"Model {model} raised exception: {e}")
        if proc.returncode is None:
            try:
                proc.kill()
            except ProcessLookupError:
                pass
        return {"error": str(e), "model": model, "success": False}


def get_model_weights() -> Dict[str, float]:
    """
    Calculates weights for each model in MODEL_POOL based on historical average duration.
    Weight = 1 / (avg_duration^2). Models without stats get a default weight.
    """
    stats_data = {}
    if AGGREGATED_STATS_FILE.exists():
        with open(AGGREGATED_STATS_FILE, "r") as f:
            try:
                # Shared lock
                fcntl.flock(f, fcntl.LOCK_SH)
                stats_data = json.load(f)
            except (json.JSONDecodeError, IOError):
                pass
            finally:
                fcntl.flock(f, fcntl.LOCK_UN)

    weights = {}
    # Use existing averages to determine a baseline for new models
    known_averages = [
        s["avg"] for s in stats_data.values() if s.get("wins", 0) > 0
    ]
    # Default to 4.5s if no stats exist, otherwise use the average of averages
    default_duration = (
        sum(known_averages) / len(known_averages) if known_averages else 4.5
    )

    # Cap the default duration to keep it competitive but not dominant
    default_duration = max(3.0, min(7.0, default_duration))

    for model in MODEL_POOL:
        stats = stats_data.get(model, {})
        avg = (
            stats.get("avg", default_duration)
            if stats.get("wins", 0) > 0
            else default_duration
        )
        # Weight follows inverse square law: 2x faster = 4x more likely
        weights[model] = 1.0 / (max(0.1, avg) ** 2)

    return weights


async def race_models(prompt: str) -> None:
    """
    Races multiple models in parallel. The first one to return a successful
    response wins. All other tasks are cancelled immediately.
    """
    # 1. Selection Logic: Prefer QUICK_PROMPT_MODEL env var, otherwise pick 2 weighted random ones.
    override = os.getenv("QUICK_PROMPT_MODEL")
    if override:
        selected_models = [override]
    else:
        weights_dict = get_model_weights()
        models = list(weights_dict.keys())
        weights = [weights_dict[m] for m in models]

        if VERBOSE:
            debug("Model weights for selection:")
            for m, w in weights_dict.items():
                debug(f"  {m}: {w:.4f}")

        # Weighted sampling without replacement
        selected_models = []
        target_count = min(len(MODEL_POOL), 2)
        temp_models = models[:]
        temp_weights = weights[:]

        while len(selected_models) < target_count and temp_models:
            choice = random.choices(temp_models, weights=temp_weights, k=1)[0]
            selected_models.append(choice)
            idx = temp_models.index(choice)
            temp_models.pop(idx)
            temp_weights.pop(idx)

    debug(f"Selected models for race: {selected_models}")

    # 2. Start parallel tasks
    tasks = [asyncio.create_task(run_model(m, prompt)) for m in selected_models]
    failed_models: Set[str] = set()

    try:
        while tasks:
            debug("Waiting for first model to complete...")
            done, pending = await asyncio.wait(
                tasks, return_when=asyncio.FIRST_COMPLETED
            )

            for winner_task in done:
                result = winner_task.result()

                if result.get("success"):
                    # We found a winner! Cancel remaining models.
                    for task in pending:
                        task.cancel()

                    # Output the winning response
                    print(result["text"])

                    # Log telemetry
                    debug(
                        f"Logging telemetry for winner: {result['model']} ({result['duration']:.3f}s)"
                    )
                    log_stats(
                        selected_models,
                        result["model"],
                        result["duration"],
                        failed_models,
                    )

                    # Flush and exit immediately to prevent any delayed output from other tasks
                    sys.stdout.flush()
                    sys.stderr.flush()
                    os._exit(0)

                else:
                    # Fail fast: if any model fails, cancel all others and exit.
                    # This ensures we don't wait for a slow model if one has already failed.
                    failed_models.add(result["model"])
                    debug(
                        f"Model {result['model']} failed: {result.get('error')}. Failing fast..."
                    )
                    for task in pending:
                        task.cancel()

                    print(
                        f"Error from {result['model']}: {result.get('error')}",
                        file=sys.stderr,
                    )

                    # Log the failure
                    log_stats(selected_models, None, 0, failed_models)

                    sys.stdout.flush()
                    sys.stderr.flush()
                    os._exit(1)

    except Exception as e:
        debug(f"Race implementation failed: {e}")
        print(f"Race failed: {e}", file=sys.stderr)
        # Clean up remaining tasks
        for task in tasks:
            task.cancel()
        sys.exit(1)


async def verify_models() -> None:
    """
    Utility to check the health and latency of all models in the pool.
    """
    print(f"Verifying {len(MODEL_POOL)} models...")
    tasks = [run_model(m, "hi") for m in MODEL_POOL]
    results = await asyncio.gather(*tasks)

    all_ok = True
    for model, res in zip(MODEL_POOL, results):
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


def main() -> None:
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

    args = parser.parse_args()

    global VERBOSE
    VERBOSE = args.verbose

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
    asyncio.run(race_models(user_prompt))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
import asyncio
import json
import fcntl
import math
import os
import random
import time
from pathlib import Path
from datetime import date, datetime

import argparse

# --- Configuration ---
STATS_DIR = Path.home() / ".local/share/quick_prompt"
DAILY_STATS_DIR = STATS_DIR / "daily"
AGGREGATED_STATS_FILE = STATS_DIR / "aggregated_stats.json"
MODEL_POOL = [
    "opencode/big-pickle",
    "opencode/gpt-5-nano",
    "opencode/mimo-v2-flash-free",
    "opencode/minimax-m2.5-free",
    "opencode/nemotron-3-super-free",
]

VERBOSE = False


def debug(msg: str):
    if VERBOSE:
        # Gray color using ANSI escape codes
        print(f"\033[90m[debug] {msg}\033[0m", file=os.sys.stderr)


async def run_model(model: str, prompt: str):
    """
    Runs a single opencode invocation and returns the result with timing.
    """
    debug(f"Starting model: {model}")
    start_time = time.perf_counter()

    # Environment setup for the subprocess
    env = os.environ.copy()
    env["OPENCODE_DISABLE_DEFAULT_PLUGINS"] = "true"
    env["OPENCODE_DISABLE_CLAUDE_CODE"] = "true"

    # Always use --variant low for each model. opencode handles models
    # that do not have this variant by falling back to the default.
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

    # Create the subprocess
    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        env=env,
    )

    try:
        # Pass the prompt and wait for result
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

        # Parse NDJSON output from opencode
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


def get_histogram_bucket(duration: float) -> str:
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


def update_aggregated_stats(selected_models, winner, duration, failed_models):
    AGGREGATED_STATS_FILE.parent.mkdir(parents=True, exist_ok=True)

    # Ensure the file exists
    if not AGGREGATED_STATS_FILE.exists():
        with open(AGGREGATED_STATS_FILE, "w") as f:
            json.dump({}, f)

    with open(AGGREGATED_STATS_FILE, "r+") as f:
        try:
            fcntl.flock(f, fcntl.LOCK_EX)
            try:
                data = json.load(f)
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

                    # Core stats
                    if stats["min"] is None or d < stats["min"]:
                        stats["min"] = d
                    stats["max"] = max(stats["max"], d)
                    stats["avg"] = sum(stats["durations"]) / len(
                        stats["durations"]
                    )

                    # Median
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

                    # Round avg for cleanliness
                    stats["avg"] = round(stats["avg"], 3)

                    # Histogram
                    bucket = get_histogram_bucket(d)
                    stats["histogram"][bucket] = (
                        stats["histogram"].get(bucket, 0) + 1
                    )

                if m in failed_models:
                    stats["fails"] += 1

            # Save back
            f.seek(0)
            f.truncate()
            json.dump(data, f, indent=2)
            f.flush()
            os.fsync(f.fileno())
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)


def log_stats(selected_models, winner, duration, failed_models):
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

    # Log to daily file (simple append-style list)
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

    # Update global aggregates with locking
    update_aggregated_stats(selected_models, winner, duration, failed_models)


async def race_models(prompt: str):
    # 1. Selection Logic
    override = os.getenv("QUICK_PROMPT_MODEL")
    selected_models = (
        [override]
        if override
        else random.sample(MODEL_POOL, min(len(MODEL_POOL), 2))
    )
    debug(f"Selected models for race: {selected_models}")

    # 2. Create Tasks
    tasks = [asyncio.create_task(run_model(m, prompt)) for m in selected_models]
    failed_models = set()

    try:
        # 3. Wait for completions
        while tasks:
            debug(f"Waiting for first model to complete...")
            done, pending = await asyncio.wait(
                tasks, return_when=asyncio.FIRST_COMPLETED
            )

            for winner_task in done:
                result = winner_task.result()

                if result.get("success"):
                    # We found a winner! Cancel everything else.
                    for task in pending:
                        task.cancel()

                    # Print the winner's text
                    print(result["text"])

                    # 5. Log Telemetry
                    debug(
                        f"Logging telemetry for winner: {result['model']} (duration={result['duration']:.3f}s)"
                    )
                    log_stats(
                        selected_models,
                        result["model"],
                        result["duration"],
                        failed_models,
                    )

                    # Flush and exit immediately
                    os.sys.stdout.flush()
                    os.sys.stderr.flush()
                    os._exit(0)

                else:
                    # Fail fast: if any model fails, cancel all others and exit
                    failed_models.add(result["model"])
                    debug(
                        f"Model {result['model']} failed: {result.get('error')}. Failing fast..."
                    )
                    for task in pending:
                        task.cancel()

                    print(
                        f"Error from {result['model']}: {result.get('error')}",
                        file=os.sys.stderr,
                    )

                    # Even on failure, we log that we tried and it failed
                    log_stats(selected_models, None, 0, failed_models)

                    os.sys.stdout.flush()
                    os.sys.stderr.flush()
                    os._exit(1)

    except Exception as e:
        debug(f"Race implementation failed: {e}")
        print(f"Race failed: {e}", file=os.sys.stderr)
        # Clean up
        for task in tasks:
            task.cancel()
        os.sys.exit(1)


async def verify_models():
    """
    Checks if each model in the pool is reachable and functional.
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
            # Truncate long error messages
            err = (err[:75] + "...") if len(err) > 75 else err
            print(f"\033[91m[FAIL]\033[0m {model}: {err}")

    if not all_ok:
        os.sys.exit(1)


if __name__ == "__main__":
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

    args = parser.parse_args()
    VERBOSE = args.verbose

    if args.ping_models:
        asyncio.run(verify_models())
        os.sys.exit(0)

    if not args.prompt:
        parser.print_help()
        os.sys.exit(1)

    user_prompt = " ".join(args.prompt)

    debug("Script initialized, starting race...")
    asyncio.run(race_models(user_prompt))

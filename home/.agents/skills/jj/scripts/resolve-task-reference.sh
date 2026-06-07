#!/usr/bin/env bash
set -euo pipefail

root=$(jj workspace root)

ref_file="$root/.task-reference"
[ -f "$ref_file" ] || {
	echo ".task-reference not found in repo root ($root)" >&2
	exit 1
}
ref=$(cat "$ref_file")
task=$(basename "$ref")
plan_dir=$(dirname "$(dirname "$ref")")
slug=$(basename "$plan_dir")
linear_json="$(dirname "$plan_dir")/.linear.json"

jq -er ".slugs[\"$slug\"].tasks[\"$task\"]" "$linear_json" || {
	echo "no linear issue for $slug/$task" >&2
	exit 1
}

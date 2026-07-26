#!/usr/bin/env bash
# create-issue.sh - create a Linear issue via `lq` and print its identifier.
#
# Usage: create-issue.sh "<title>" [team-name]
#
# Team defaults to `default_team_name` from ~/.config/linear (or R&D). Assigns
# the issue to the current user and sets its state to "In Progress". Mirrors
# `just create-issue --non-interactive --skip-branch`, but uses `lq` so it works
# from any repo/shell (no monorepo or fish-config dependency).
#
# On success prints the issue identifier (e.g. `HS-1234`) to stdout.
# Exits 2 if the team name is not found; 1 on any API error.
set -euo pipefail

title=${1:?usage: create-issue.sh "<title>" [team-name]}
team=${2:-$(jq -r '.default_team_name // "R&D"' "${LINEAR_CONFIG:-$HOME/.config/linear}")}

# Run a query through lq; on failure echo lq's (captured) output to stderr.
q() {
	local out
	if ! out=$(lq "$1"); then
		printf '%s\n' "$out" >&2
		return 1
	fi
	printf '%s\n' "$out"
}

# One call for viewer id + all teams (id, name, key).
viewer=$(q '{ viewer { id teams { nodes { id name key } } } }')
user_id=$(printf '%s' "$viewer" | jq -r '.data.viewer.id')

team_node=$(printf '%s' "$viewer" | jq -c --arg n "$team" \
	'.data.viewer.teams.nodes[] | select(.name == $n)')
if [ -z "$team_node" ]; then
	echo "create-issue: team '$team' not found in your Linear workspace." >&2
	exit 2
fi
team_id=$(printf '%s' "$team_node" | jq -r '.id')

# "In Progress" workflow state for this team.
state_id=$(q "{ workflowStates(filter: { team: { id: { eq: \"$team_id\" } }, name: { eq: \"In Progress\" } }) { nodes { id } } }" |
	jq -r '.data.workflowStates.nodes[0].id // empty')
if [ -z "$state_id" ]; then
	echo "create-issue: no 'In Progress' state for team '$team'." >&2
	exit 1
fi

# JSON-encode the title into a valid GraphQL string literal (JSON string
# escaping is a subset GraphQL accepts: \" \\ \n \t \uXXXX ...).
title_lit=$(jq -n --arg t "$title" '$t')

result=$(q "mutation { issueCreate(input: { title: $title_lit, teamId: \"$team_id\", stateId: \"$state_id\", assigneeId: \"$user_id\" }) { success issue { identifier } } }")

if [ "$(printf '%s' "$result" | jq -r '.data.issueCreate.success')" != "true" ]; then
	printf '%s\n' "$result" >&2
	echo "create-issue: issueCreate did not succeed." >&2
	exit 1
fi

printf '%s' "$result" | jq -r '.data.issueCreate.issue.identifier'

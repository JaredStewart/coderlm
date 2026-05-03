#!/bin/bash
# plugin/scripts/session-stop.sh
# Save annotations and clean up coderlm session on Stop.
# Called by the Stop hook — must never block or fail loudly.

PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
CLI="$PLUGIN_ROOT/skills/coderlm/scripts/coderlm_cli.py"
STATE_FILE=".claude/coderlm_state/session.json"
PORT="${CODERLM_PORT:-3000}"

if [ -f "$STATE_FILE" ] && curl -s --max-time 2 "http://127.0.0.1:${PORT}/api/v1/health" > /dev/null 2>&1; then
    python3 "$CLI" save-annotations 2>/dev/null || true
    python3 "$CLI" cleanup 2>/dev/null || true
fi

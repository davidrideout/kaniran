#!/bin/bash
# Deploy the ichiran trace harness to a remote host and (re)start the pooled
# uvicorn API.
#
# Usage:
#   REMOTE_HOST=user@host.example ./deploy_server.sh \
#       [--pool-size N] [--no-deploy] [--no-restart]
#
# NB: the script exits in ~10s for a full restart (pool boots from
# ichiran.core in ~300ms, wait_healthy.py succeeds on the first or second
# poll). DO NOT pipe through `tail -N` or `head -N` — those tools buffer
# until stdin closes, so output appears as a single chunk at exit and the
# script LOOKS hung even though it isn't. Run it in the foreground, or pipe
# through `tee` / `cat` if you need a copy.
#
# Required env (or override the defaults below):
#   REMOTE_HOST       SSH target, e.g. user@host.example
#   REMOTE_API_DIR    Directory on the remote to receive the .lisp files
#   REMOTE_STORAGE    Directory on the remote that holds ichiran.core + logs
#   REMOTE_HOME       Remote user's home (used for the fetcher script)
#
# Defaults:
#   --pool-size  18    (passed as ICHIRAN_POOL_SIZE)
#
# Steps (run in order, all skippable):
#   1. Deploy: scp the 3 Lisp files + fetch_traces_full.py
#   2. Restart pool: kill old uvicorn+sbcl; relaunch with desired pool size
#   3. Health check: poll /health from local until all workers are healthy
#      or 5min timeout.
#
# This script ONLY deploys assets and stands up the server. It does NOT run
# extractions — those are launched manually with nohup+disown.

set -euo pipefail

REMOTE="${REMOTE_HOST:-user@example}"
REMOTE_API_DIR="${REMOTE_API_DIR:-/home/$(echo "$REMOTE" | cut -d@ -f1)/pooled-api}"
REMOTE_STORAGE="${REMOTE_STORAGE:-/home/$(echo "$REMOTE" | cut -d@ -f1)/storage}"
REMOTE_HOME="${REMOTE_HOME:-/home/$(echo "$REMOTE" | cut -d@ -f1)}"
REMOTE_HOST_ONLY="$(echo "$REMOTE" | cut -d@ -f2)"
HEALTH_URL="${HEALTH_URL:-http://${REMOTE_HOST_ONLY}:9100/health}"

POOL_SIZE=18
DO_DEPLOY=1
DO_RESTART=1

while [[ $# -gt 0 ]]; do
    case "$1" in
        --pool-size)  POOL_SIZE="$2"; shift 2 ;;
        --no-deploy)  DO_DEPLOY=0; shift ;;
        --no-restart) DO_RESTART=0; shift ;;
        -h|--help)    sed -n '2,26p' "$0"; exit 0 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Plan ==="
echo "  remote=$REMOTE"
echo "  pool_size=$POOL_SIZE"
echo "  steps: deploy=$DO_DEPLOY restart=$DO_RESTART"
echo

# 1. Deploy ────────────────────────────────────────────────────────────────────
if (( DO_DEPLOY )); then
    echo "=== [1/3] Deploy harness + driver to $REMOTE ==="
    scp "$SCRIPT_DIR/trace_capture.lisp" \
        "$SCRIPT_DIR/extractor_worker.lisp" \
        "$SCRIPT_DIR/ichiran_main_pooled.py" \
        "$SCRIPT_DIR/ichiran_worker_pool.py" \
        "$REMOTE:$REMOTE_API_DIR/"
    scp "$SCRIPT_DIR/fetch_extractor.py" "$REMOTE:$REMOTE_HOME/"
    ssh "$REMOTE" "ls -la $REMOTE_API_DIR/{trace_capture,extractor_worker}.lisp $REMOTE_API_DIR/ichiran_{main_pooled,worker_pool}.py $REMOTE_HOME/fetch_extractor.py"
    echo
fi

# 2. Restart pool ──────────────────────────────────────────────────────────────
if (( DO_RESTART )); then
    echo "=== [2/3] Restart uvicorn pool (size=$POOL_SIZE) ==="
    # Kill in dependency order: fetcher → uvicorn parent → SBCL workers.
    # Patterns are anchored to ARG0 (^python|^sbcl) so the bash shell running
    # the script itself doesn't match. Two passes — second catches respawns.
    ssh "$REMOTE" '
      kill_pat() {
        local pids
        pids=$(pgrep -f -- "$1" 2>/dev/null || true)
        [ -n "$pids" ] && kill -9 $pids 2>/dev/null || true
      }
      kill_pat "^python.*fetch_extractor"
      kill_pat "^python.*ichiran_main_pooled"
      kill_pat "^sbcl.*extractor_worker"
      sleep 2
      kill_pat "^python.*fetch_extractor"
      kill_pat "^python.*ichiran_main_pooled"
      kill_pat "^sbcl.*extractor_worker"
      true
    '
    sleep 2
    if ssh "$REMOTE" "ss -tln | grep -q ':9100 '"; then
        echo "  WARN: port 9100 still bound; killing remaining listener" >&2
        ssh "$REMOTE" "fuser -k -9 9100/tcp 2>/dev/null || true"
        sleep 1
    fi
    # `ssh -n -f` is the documented daemon-launch pattern: -n redirects local
    # stdin from /dev/null, -f forks ssh to the background just before exec,
    # so the local script returns immediately without waiting on the remote
    # uvicorn. Subshell-detach (`( ... & )`) and `& disown` both still hang
    # because the spawned daemon's fds keep the ssh channel pinned open.
    ssh -n -f "$REMOTE" "cd $REMOTE_API_DIR && \
      ICHIRAN_SBCL_CORE=$REMOTE_STORAGE/ichiran.core \
      ICHIRAN_WORKER_SCRIPT=$REMOTE_API_DIR/extractor_worker.lisp \
      ICHIRAN_POOL_SIZE=$POOL_SIZE \
      nohup python3 -m uvicorn ichiran_main_pooled:app \
        --host 0.0.0.0 --port 9100 \
        > $REMOTE_STORAGE/pooled-api.log 2>&1 < /dev/null"
    echo "  uvicorn launched (ssh -n -f)"
    echo
fi

# 3. Health check ──────────────────────────────────────────────────────────────
echo "=== [3/3] Wait for $POOL_SIZE healthy workers ==="
python3 "$SCRIPT_DIR/wait_healthy.py" --url "$HEALTH_URL" --timeout 300 --interval 3
echo

echo "=== Done ==="

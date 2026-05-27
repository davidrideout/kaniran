#!/bin/bash
# run-remote.sh
#
# Deterministic pipeline:
#   1. scp introspect.lisp to the remote
#   2. run sbcl on the remote, which writes /tmp/ichiran-reverse/
#   3. rsync the result back into reverse/ (preserving reverse/scripts/)
#
# Env overrides:
#   ICHIRAN_REMOTE   default user@ichiran-host
#   ICHIRAN_REMOTE_DIR   default /storage/ichiran
#   ICHIRAN_REMOTE_OUT   default /tmp/ichiran-reverse

set -euo pipefail

HOST="${ICHIRAN_REMOTE:-user@ichiran-host}"
REMOTE_DIR="${ICHIRAN_REMOTE_DIR:-/path/to/storage/ichiran}"
REMOTE_OUT="${ICHIRAN_REMOTE_OUT:-/tmp/ichiran-reverse}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOCAL_REVERSE="$(cd "$SCRIPT_DIR/.." && pwd)"

echo ">> [1/3] Copying introspect.lisp to ${HOST}:/tmp/ichiran-introspect.lisp"
scp "$SCRIPT_DIR/introspect.lisp" "${HOST}:/tmp/ichiran-introspect.lisp"

echo ">> [2/3] Running SBCL introspection on ${HOST} (cwd=${REMOTE_DIR})"
ssh "$HOST" "set -e; cd '${REMOTE_DIR}'; rm -rf '${REMOTE_OUT}'; sbcl --non-interactive --load /tmp/ichiran-introspect.lisp -- '${REMOTE_OUT}'"

echo ">> [3/3] Rsyncing ${HOST}:${REMOTE_OUT}/ -> ${LOCAL_REVERSE}/"
rsync -av --delete --exclude='scripts/' "${HOST}:${REMOTE_OUT}/" "${LOCAL_REVERSE}/"

echo
echo ">> Done. Markdown file count:"
find "$LOCAL_REVERSE" -name '*.md' -not -path '*/scripts/*' | wc -l

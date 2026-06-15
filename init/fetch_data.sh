#!/usr/bin/env bash
#
# Download the source dictionaries kaniran's loaders build from:
#   - JMdict_e  (JMdict, English edition)  ->  data/JMdict_e.xml
#   - kanjidic2 (kanji database)           ->  data/kanjidic2.xml
#
# Both are published gzipped by the EDRDG (http://www.edrdg.org/); this
# fetches and decompresses them into data/ at the repository root.
#
# Usage:
#   init/fetch_data.sh [--force]
#       --force   re-download even if the .xml already exists
#
# Environment overrides:
#   DATA_DIR        destination directory       (default: <repo>/data)
#   JMDICT_URL      JMdict_e source .gz URL     (default: EDRDG current)
#   KANJIDIC_URL    kanjidic2 source .gz URL    (default: EDRDG current)
#   To pin a reproducible build, point these at a dated mirror/snapshot.
#
# Feed the results to the loader:
#   cargo run --release -p kaniran-loader --bin full_e2e_load -- \
#       --db-url postgres:///kaniran \
#       --jmdict-path   data/JMdict_e.xml \
#       --kanjidic-path data/kanjidic2.xml
#
# JMdict and kanjidic2 are the property of EDRDG, used under its licence
# (CC BY-SA 4.0): http://www.edrdg.org/edrdg/licence.html

set -euo pipefail

JMDICT_URL="${JMDICT_URL:-http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz}"
KANJIDIC_URL="${KANJIDIC_URL:-http://www.edrdg.org/kanjidic/kanjidic2.xml.gz}"

# Repo root is the parent of this script's directory (init/).
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
data_dir="${DATA_DIR:-$repo_root/data}"

usage() {
    sed -n '3,28p' "${BASH_SOURCE[0]}" | sed 's/^#\{1,\} \{0,1\}//; s/^#$//'
}

force=0
for argument in "$@"; do
    case "$argument" in
        --force) force=1 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "fetch_data: unknown argument: $argument" >&2; usage >&2; exit 2 ;;
    esac
done

# Write <url> to <dest> with whichever downloader is installed.
download() {
    local url="$1" dest="$2"
    if command -v curl >/dev/null 2>&1; then
        curl -fL --retry 3 --retry-delay 2 -o "$dest" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "$dest" "$url"
    else
        echo "fetch_data: need curl or wget on PATH" >&2
        exit 1
    fi
}

# Download <url>, gunzip it to data_dir/<name>. Skips when <name> already
# exists unless --force.
fetch() {
    local url="$1" name="$2"
    local out="$data_dir/$name"
    if [[ -s "$out" && "$force" -ne 1 ]]; then
        echo "skip   $name (already present; pass --force to refresh)"
        return
    fi
    local gz="$data_dir/$name.gz"
    echo "fetch  $name  <-  $url"
    download "$url" "$gz"
    echo "expand $name"
    gunzip -c "$gz" > "$out"
    rm -f "$gz"
    echo "ok     $out ($(wc -c < "$out" | tr -d ' ') bytes)"
}

mkdir -p "$data_dir"
fetch "$JMDICT_URL"   "JMdict_e.xml"
fetch "$KANJIDIC_URL" "kanjidic2.xml"

echo
echo "dictionaries ready in $data_dir"

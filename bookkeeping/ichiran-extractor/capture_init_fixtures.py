#!/usr/bin/env python3
"""Drive capture_init_fixtures.lisp on a remote ichiran host and write
the captures as per-FQN parquet under <output_dir>/<package>/<symbol>.parquet
(same layout fetch_extractor.py uses for corpus runs).

Why this exists: a few wave-110-onward symbols (notably get-kana-form)
are only called inside ichiran/dict:init-suffixes — never at romanize*
runtime. The pooled corpus extractor can't see them because the pool
runs init at boot, before /install lands. This driver runs init in a
fresh SBCL with the tracer pre-installed and a forced reset.

Usage:
    python3 capture_init_fixtures.py \\
        --host user@ichiran-host \\
        --remote-dir /path/to/pooled-api \\
        --core /path/to/storage/ichiran.core \\
        --output corpus/extracted/

The lisp script writes a single JSON envelope to stdout
({"captures":[...], "skipped":N}). This driver parses it, groups by
fn, and writes one parquet per fn with the same schema and metadata
fields the corpus fetcher emits.
"""
import argparse
import json
import os
import subprocess
import sys
from datetime import datetime, timezone

import pyarrow as pa
import pyarrow.parquet as pq

SCHEMA = pa.schema([
    pa.field("args", pa.string()),
    pa.field("result", pa.string()),
])


def fqn_to_path(fqn: str) -> tuple[str, str]:
    """Replicate fetch_extractor.fqn_to_path: PKG:SYM -> ('pkg', 'sym').
    Bare ICHIRAN: lands under 'core'."""
    if "::" in fqn:
        pkg_part, sym = fqn.split("::", 1)
    elif ":" in fqn:
        pkg_part, sym = fqn.split(":", 1)
    else:
        raise ValueError(f"no package separator in {fqn!r}")
    pkg = pkg_part.split("/", 1)[1] if "/" in pkg_part else "core"
    sym_file = sym.lower().replace("-", "_").replace("*", "_star_").replace("+", "_plus_")
    return pkg.lower(), sym_file


def main():
    p = argparse.ArgumentParser(description=__doc__.strip().splitlines()[0])
    p.add_argument("--host", required=True, help="ssh target, e.g. user@ichiran-host")
    p.add_argument("--remote-dir", default="/path/to/pooled-api",
                   help="dir on host containing capture_init_fixtures.lisp + projectors.lisp + trace_capture.lisp")
    p.add_argument("--core", default="/path/to/storage/ichiran.core",
                   help="path to ichiran.core on the host")
    p.add_argument("--output", required=True,
                   help="local output dir; per-fqn parquet lands at <output>/<pkg>/<sym>.parquet")
    args = p.parse_args()

    cmd = [
        "ssh", args.host,
        f"sbcl --core {args.core} --noinform --non-interactive "
        f"--load {args.remote_dir}/capture_init_fixtures.lisp",
    ]
    print(f"running: {' '.join(cmd)}", file=sys.stderr)
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
    if proc.returncode != 0:
        sys.exit(f"sbcl failed (rc={proc.returncode}):\nstdout:\n{proc.stdout}\nstderr:\n{proc.stderr}")
    if proc.stderr.strip():
        print(f"sbcl stderr: {proc.stderr.strip()}", file=sys.stderr)

    try:
        envelope = json.loads(proc.stdout.strip().splitlines()[-1])
    except (json.JSONDecodeError, IndexError) as e:
        sys.exit(f"failed to parse JSON envelope from sbcl stdout:\n{proc.stdout}\n\n{e}")

    captures = envelope.get("captures", [])
    if not captures:
        sys.exit(f"no captures returned (skipped={envelope.get('skipped')})")
    print(f"received {len(captures)} captures, {envelope.get('skipped')} skipped", file=sys.stderr)

    # Group by fn.
    by_fn: dict[str, list[tuple[str, str]]] = {}
    for c in captures:
        by_fn.setdefault(c["fn"], []).append((c["args"], c["result"]))

    metadata_str = {
        "captured_at": datetime.now(timezone.utc).isoformat(),
        "driver": "capture_init_fixtures.py",
        "ichiran_rev": os.environ.get("ICHIRAN_REV", "unknown"),
        "input": "init-suffixes",
    }

    for fqn, rows in by_fn.items():
        pkg_dir, sym_file = fqn_to_path(fqn)
        out_dir = os.path.join(args.output, pkg_dir)
        os.makedirs(out_dir, exist_ok=True)
        out_path = os.path.join(out_dir, f"{sym_file}.parquet")
        meta = {b"ichiran_extractor_fqn": fqn.encode("utf-8")}
        for k, v in metadata_str.items():
            meta[k.encode("utf-8")] = str(v).encode("utf-8")
        schema_with_meta = SCHEMA.with_metadata(meta)
        table = pa.Table.from_pydict(
            {"args": [r[0] for r in rows], "result": [r[1] for r in rows]},
            schema=schema_with_meta,
        )
        pq.write_table(table, out_path)
        print(f"  wrote {len(rows):>4} rows -> {out_path}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Drive the ichiran extractor pool over a sentence corpus and write
per-FQN parquet fixtures under <output_dir>/<package>/<symbol>.parquet.

Pipeline:
    producer    streams sentences into a bounded asyncio.Queue
    workers     N coroutines, each calls POST /extract per sentence,
                pushes (fn, args, result) tuples downstream
    writer      single task; per-FQN dedup-by-args + per-FQN parquet
                writer kept open for the whole run

Schema per parquet file:
    args   : utf8     (Lisp source text of the args list)
    result : utf8     (Lisp source text of the multiple-value-list result)
Arrow file metadata embeds run provenance:
    ichiran_rev    git rev of ichiran on the worker host (read once at startup)
    captured_at    ISO-8601 timestamp at run start
    driver         git rev of this script (best-effort)

Usage:
    # Install hooks once on the pool (one FQN per line in fqns.txt):
    python3 fetch_extractor.py install fqns.txt --api http://host:9100

    # Run the fetch (input defaults to ../corpus/diverse_250k_2026_05_09.parquet):
    python3 fetch_extractor.py fetch corpus/extracted \\
        --api http://host:9100 --workers 18 [--limit 250000] [--skip 0]

    # Override the corpus:
    python3 fetch_extractor.py fetch corpus/extracted --input other.parquet ...

Input format is detected by extension:
- .parquet: read the `text` column (schema from build_diverse_corpus.py:
  number / source / text). Rows iterated in file order.
- otherwise: parsed Tatoeba-style (tab-split, sentence in column 3 if
  present, otherwise the whole line — also handles plain
  one-sentence-per-line).
"""

import argparse
import asyncio
import json
import os
import sys
import time
from datetime import datetime, timezone
from typing import Optional

import aiohttp
import orjson
import pyarrow as pa
import pyarrow.parquet as pq

DEFAULT_API = "http://localhost:9100"
DEFAULT_CORPUS = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "corpus", "diverse_250k_2026_05_09.parquet",
)
HTTP_TIMEOUT = 120
MAX_RETRIES = 5
BACKOFF_BASE = 2
SENTINEL = None  # poison pill

SCHEMA = pa.schema([
    pa.field("args", pa.string()),
    pa.field("result", pa.string()),
])


def eprint(*args, **kwargs):
    print(*args, file=sys.stderr, flush=True, **kwargs)


def fqn_to_path(fqn: str) -> tuple[str, str]:
    """Map 'ICHIRAN/CHARACTERS:NORMALIZE' to ('characters', 'normalize').
    Translation rules are the same kani::naming uses on the Rust side
    (subset: package qualifier becomes the directory; bare 'ichiran'
    becomes 'core'; symbol name is lowercased and *-+ get the same
    sub-replacements). The exhaustive rules live in
    kaniran-core/src/kani/naming.rs."""
    if ":" not in fqn:
        raise ValueError(f"FQN missing package separator: {fqn}")
    pkg, sym = fqn.split(":", 1)
    if sym.startswith(":"):
        sym = sym[1:]
    pkg = pkg.lower()
    if pkg.startswith("ichiran/"):
        pkg_dir = pkg[len("ichiran/"):]
    elif pkg == "ichiran":
        pkg_dir = "core"
    else:
        pkg_dir = pkg.replace("/", "_")
    sym_file = (
        sym.lower()
        .replace("*", "_star_")
        .replace("+", "_plus_")
        .replace("-", "_")
    )
    while "__" in sym_file:
        sym_file = sym_file.replace("__", "_")
    return pkg_dir, sym_file


# --- HTTP helpers ------------------------------------------------------------

async def post_json(session, url, body, timeout=HTTP_TIMEOUT):
    # Encode the request body with orjson; aiohttp's `json=body` would
    # use stdlib json. Decode the response body with orjson too — for
    # multi-MB /extract envelopes that's the hot loop.
    payload = orjson.dumps(body)
    headers = {"Content-Type": "application/json"}
    for attempt in range(MAX_RETRIES):
        try:
            async with session.post(
                url,
                data=payload,
                headers=headers,
                timeout=aiohttp.ClientTimeout(total=timeout),
            ) as resp:
                if resp.status >= 500:
                    raise RuntimeError(f"server {resp.status}")
                raw = await resp.read()
                return orjson.loads(raw)
        except Exception:
            if attempt == MAX_RETRIES - 1:
                raise
            await asyncio.sleep(BACKOFF_BASE * (2 ** attempt))


# --- producer ----------------------------------------------------------------

def iter_sentences(input_file: str, skip: int, limit: Optional[int]):
    if input_file.lower().endswith(".parquet"):
        yield from _iter_parquet(input_file, skip, limit)
    else:
        yield from _iter_text(input_file, skip, limit)


def _iter_text(input_file: str, skip: int, limit: Optional[int]):
    idx = 0
    seen = 0
    with open(input_file, encoding="utf-8") as f:
        for row in f:
            row = row.strip()
            if not row:
                continue
            seen += 1
            if seen <= skip:
                continue
            parts = row.split("\t")
            sentence = parts[2] if len(parts) >= 3 else row
            idx += 1
            yield sentence
            if limit and idx >= limit:
                return


def _iter_parquet(input_file: str, skip: int, limit: Optional[int]):
    pf = pq.ParquetFile(input_file)
    if "text" not in pf.schema_arrow.names:
        raise ValueError(
            f"{input_file}: parquet missing 'text' column "
            f"(found {pf.schema_arrow.names})"
        )
    idx = 0
    seen = 0
    for batch in pf.iter_batches(batch_size=10000, columns=["text"]):
        for value in batch.column("text"):
            sentence = value.as_py()
            if not sentence:
                continue
            sentence = sentence.strip()
            if not sentence:
                continue
            seen += 1
            if seen <= skip:
                continue
            idx += 1
            yield sentence
            if limit and idx >= limit:
                return


async def producer(input_queue, input_file, skip, limit, n_workers):
    for sentence in iter_sentences(input_file, skip, limit):
        await input_queue.put(sentence)
    for _ in range(n_workers):
        await input_queue.put(SENTINEL)


# --- worker ------------------------------------------------------------------

async def worker(
    wid,
    input_queue,
    output_queue,
    session,
    api,
    state,
    progress_every,
):
    """Pull sentences off input_queue, POST /extract, fan captures
    onto output_queue. After each sentence completes, bump the shared
    sentence counter and emit a progress line every PROGRESS_EVERY
    sentences. asyncio is single-threaded so the counter is race-free."""
    extract_url = f"{api.rstrip('/')}/extract"
    while True:
        sentence = await input_queue.get()
        if sentence is SENTINEL:
            input_queue.task_done()
            break
        try:
            result = await post_json(session, extract_url, {"text": sentence})
        except Exception as exc:
            eprint(f"worker {wid}: extract failed for {sentence!r}: {exc}")
            state["sentences"] += 1
            state["sentences_failed"] += 1
            input_queue.task_done()
            _maybe_log_progress(state, progress_every)
            continue
        captures = result.get("captures", []) if isinstance(result, dict) else []
        for cap in captures:
            await output_queue.put((cap["fn"], cap["args"], cap["result"]))
            state["captures"] += 1
        state["sentences"] += 1
        input_queue.task_done()
        _maybe_log_progress(state, progress_every)


def _maybe_log_progress(state, progress_every):
    n = state["sentences"]
    if n == 0 or n % progress_every != 0:
        return
    elapsed = time.time() - state["t0"]
    rate = n / elapsed if elapsed > 0 else 0
    total = state.get("total") or 0
    cap = state["captures"]
    cap_rate = cap / elapsed if elapsed > 0 else 0
    if total and total > n and rate > 0:
        eta_sec = (total - n) / rate
        eta = f", ETA {eta_sec / 60:.1f}min" if eta_sec >= 60 else f", ETA {eta_sec:.0f}s"
        pct = f"{100 * n / total:5.1f}%"
    else:
        eta = ""
        pct = "  --  "
    failed = state["sentences_failed"]
    fail_str = f", failed={failed}" if failed else ""
    print(
        f"  {n:>9,}/{total:,} sentences ({pct}, {rate:>5.0f} sent/s{eta}), "
        f"captures={cap:>10,} ({cap_rate:>6.0f}/s){fail_str}",
        flush=True,
    )


# --- writer ------------------------------------------------------------------

class FqnWriter:
    """One ParquetWriter per FQN. Buffers rows and flushes a row group
    every BATCH rows. No dedup — duplicates land in the parquet and are
    collapsed post-run with duckdb. (Earlier per-FQN dedup sets blew the
    driver to ~22 GB anon RSS on a 250k-sentence corpus and OOM-killed
    the process.)"""

    BATCH = 4096

    def __init__(self, out_dir: str, fqn: str, metadata: dict):
        pkg_dir, sym_file = fqn_to_path(fqn)
        os.makedirs(os.path.join(out_dir, pkg_dir), exist_ok=True)
        self.path = os.path.join(out_dir, pkg_dir, f"{sym_file}.parquet")
        # arrow file-level KV metadata is bytes-keyed.
        self.metadata = {b"ichiran_extractor_fqn": fqn.encode("utf-8")}
        for k, v in metadata.items():
            self.metadata[k.encode("utf-8")] = str(v).encode("utf-8")
        self.schema = SCHEMA.with_metadata(self.metadata)
        self.writer = pq.ParquetWriter(
            self.path, self.schema,
            compression="zstd", compression_level=3,
        )
        self.args_buf: list[str] = []
        self.results_buf: list[str] = []
        self.n_written = 0

    def add(self, args: str, result: str):
        self.args_buf.append(args)
        self.results_buf.append(result)
        if len(self.args_buf) >= self.BATCH:
            self._flush()

    def _flush(self):
        if not self.args_buf:
            return
        table = pa.table(
            {"args": self.args_buf, "result": self.results_buf},
            schema=self.schema,
        )
        self.writer.write_table(table)
        self.n_written += len(self.args_buf)
        self.args_buf.clear()
        self.results_buf.clear()

    def close(self):
        self._flush()
        self.writer.close()


async def writer_task(
    output_queue,
    out_dir: str,
    n_workers: int,
    metadata: dict,
):
    writers: dict[str, FqnWriter] = {}
    workers_done = 0
    n_captures = 0
    t0 = time.time()
    try:
        while True:
            item = await output_queue.get()
            if item is SENTINEL:
                workers_done += 1
                output_queue.task_done()
                if workers_done >= n_workers:
                    break
                continue
            fqn, args, result = item
            w = writers.get(fqn)
            if w is None:
                w = FqnWriter(out_dir, fqn, metadata)
                writers[fqn] = w
            w.add(args, result)
            n_captures += 1
            output_queue.task_done()
    finally:
        for w in writers.values():
            w.close()
    elapsed = time.time() - t0
    print(
        f"\nDone: {n_captures} captures in {elapsed:.0f}s, "
        f"{len(writers)} fqns. Per fqn:",
        flush=True,
    )
    for fqn, w in sorted(writers.items()):
        print(f"  {fqn}  written={w.n_written}", flush=True)


# --- subcommands -------------------------------------------------------------

async def cmd_install(args):
    # Each non-blank, non-comment line is either:
    #   FQN
    #   FQN  PROJECTOR-NAME
    # Whitespace-delimited, two columns max. PROJECTOR-NAME resolves
    # via :ichi-projectors on the worker (see projectors.lisp).
    specs: list[str | dict[str, str]] = []
    with open(args.fqn_file, encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            if len(parts) == 1:
                specs.append(parts[0])
            elif len(parts) == 2:
                specs.append({"fqn": parts[0], "result_projector": parts[1]})
            else:
                sys.exit(f"bad line (expected FQN [projector]): {line!r}")
    if not specs:
        sys.exit("no FQNs in file")
    n_proj = sum(1 for s in specs if isinstance(s, dict))
    print(f"installing {len(specs)} FQNs on {args.api} ({n_proj} with projector)")
    async with aiohttp.ClientSession() as session:
        result = await post_json(session, f"{args.api.rstrip('/')}/install", {"fqns": specs})
    print(json.dumps(result, indent=2))


async def cmd_clear(args):
    async with aiohttp.ClientSession() as session:
        result = await post_json(session, f"{args.api.rstrip('/')}/clear", {})
    print(json.dumps(result, indent=2))


async def cmd_fetch(args):
    os.makedirs(args.output, exist_ok=True)
    metadata = {
        "captured_at": datetime.now(timezone.utc).isoformat(),
        "driver": os.environ.get("EXTRACTOR_DRIVER_REV", "unknown"),
        "ichiran_rev": os.environ.get("ICHIRAN_REV", "unknown"),
        "input": args.input,
    }
    # Pre-count expected sentences for ETA. Counts only post-skip lines
    # up to limit. Cheap for plain text — even 2M lines is sub-second.
    total_sentences = 0
    for _ in iter_sentences(args.input, args.skip, args.limit):
        total_sentences += 1
    print(f"writing fixtures to {args.output}/, workers={args.workers}, api={args.api}")
    print(f"metadata: {metadata}")
    print(f"sentences to process: {total_sentences:,} "
          f"(progress every {args.progress_every:,})")

    state = {
        "sentences": 0,
        "sentences_failed": 0,
        "captures": 0,
        "total": total_sentences,
        "t0": time.time(),
    }

    input_queue: asyncio.Queue = asyncio.Queue(maxsize=args.workers * 2)
    output_queue: asyncio.Queue = asyncio.Queue(maxsize=args.workers * 8)
    connector = aiohttp.TCPConnector(limit=args.workers, limit_per_host=args.workers)
    async with aiohttp.ClientSession(connector=connector) as session:
        prod = asyncio.create_task(
            producer(input_queue, args.input, args.skip, args.limit, args.workers)
        )
        wkrs = [
            asyncio.create_task(worker(
                i, input_queue, output_queue, session, args.api,
                state, args.progress_every,
            ))
            for i in range(args.workers)
        ]
        wtask = asyncio.create_task(
            writer_task(output_queue, args.output, args.workers, metadata)
        )
        await prod
        await asyncio.gather(*wkrs)
        for _ in range(args.workers):
            await output_queue.put(SENTINEL)
        await wtask


def main():
    desc = (__doc__ or "").strip().splitlines()[0] if __doc__ else "ichiran extractor driver"
    parser = argparse.ArgumentParser(description=desc)
    parser.add_argument("--api", default=DEFAULT_API)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_inst = sub.add_parser("install", help="install hooks on the pool")
    p_inst.add_argument("fqn_file", help="text file, one FQN per line")

    sub.add_parser("clear", help="clear capture buffers on the pool")

    p_fetch = sub.add_parser("fetch", help="drive corpus through pool, write parquet")
    p_fetch.add_argument(
        "--input", default=DEFAULT_CORPUS,
        help=(
            "input corpus. .parquet (text column) or TSV/text "
            "(sentence in col 3, or whole line). "
            f"Default: {DEFAULT_CORPUS}"
        ),
    )
    p_fetch.add_argument("output", help="output dir for per-FQN parquet files")
    p_fetch.add_argument("--workers", type=int, default=4)
    p_fetch.add_argument("--skip", type=int, default=0)
    p_fetch.add_argument("--limit", type=int, default=0,
                         help="max sentences (0 = all)")
    p_fetch.add_argument("--progress-every", type=int, default=10000,
                         help="emit a progress line every N sentences (default 10000)")

    args = parser.parse_args()
    if args.cmd == "fetch":
        args.limit = args.limit or None
        asyncio.run(cmd_fetch(args))
    elif args.cmd == "install":
        asyncio.run(cmd_install(args))
    elif args.cmd == "clear":
        asyncio.run(cmd_clear(args))


if __name__ == "__main__":
    main()

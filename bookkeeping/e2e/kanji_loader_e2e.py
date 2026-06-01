#!/usr/bin/env python3
"""End-to-end parity test for kaniran's kanjidic loaders.

Compares kaniran's `load_kanjidic` output against the canonical ichiran
release `ichiran-260118` (the pgdump tshatrov/ichiran shipped on
2026-01-18, restored locally as DB `ichiran_260118`). The kanjidic2
input matched empirically to that pgdump is the 2015-03-17 snapshot
(`database_version=2015-076`) — although JMdict was refreshed for the
2026-01-18 release, the kanji half of ichiran was loaded eleven years
prior and never refreshed. Identified by walking Wayback snapshots
(2026 / 2019 / 2017 / 2016 / 2015) and matching divergent rows (`極`
stroke-count order, `飈` classical radical 181 vs later 182, `䀹`
meaning preserving the upstream "smller" typo, `串` carrying the
"shish kebab" gloss later removed).

Provenance / SHAs:
  ichiran-260118.pgdump
    https://github.com/tshatrov/ichiran/releases/download/ichiran-260118/ichiran-260118.pgdump
    sha256 98a44e2cc88a65677da8b1f7124e7d6c904253eb1aae0ef16d2c7cc1dacdba82
    restore:
      dropdb --if-exists ichiran_260118 && createdb ichiran_260118
      pg_restore --no-owner --no-privileges -d ichiran_260118 ichiran-260118.pgdump
  kanjidic2_2015-03-17.xml (canonical input — reproduces the pgdump's kanji)
    http://web.archive.org/web/20150317225430/http://www.edrdg.org/kanjidic/kanjidic2.xml.gz
    database_version=2015-076, date_of_creation=2015-03-17
  kanjidic2_2019-03-11.xml / kanjidic2_2026-01-10.xml (kept for re-baselining)

The script drops + creates the test database, applies `db/schema.sql`,
optionally copies the JMdict-derived tables (entry, kanji_text,
kana_text) from the reference DB so load_kanji_stats can run, invokes
the Rust `e2e_load_kanjidic` binary, then stream-compares the four
kanjidic tables (kanji, reading, okurigana, meaning) between the
reference and test DBs.

The comparison is **streaming**: both DBs export rows sorted by natural
key via `psql \\COPY ... TO STDOUT CSV`; the merge walks the two streams
in lockstep holding at most one row from each side at a time, so memory
is O(1) in row count. All mismatches are dumped to the report file.

Usage:
  python3 bookkeeping/e2e/kanji_loader_e2e.py \\
      [--ref-db ichiran_260118] \\
      [--test-db ichiran_kanji_e2e] \\
      [--kanjidic-path bookkeeping/e2e/fixtures/kanjidic2_2026-01-10.xml] \\
      [--with-stats]     # also copy JMdict tables + run load_kanji_stats
      [--skip-load]      # reuse the existing test-DB state
      [--skip-drop]      # don't drop+create; assume schema already there
      [--report bookkeeping/e2e/last_run.md]
"""

from __future__ import annotations

import argparse
import csv
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional

REPO_ROOT = Path(__file__).resolve().parent.parent.parent


@dataclass
class TableCompare:
    """One table to diff between the two DBs.

    `select_cols` is the projection (DB-qualified column expressions).
    `query` is the full SELECT, ORDER BY identical to select_cols so the
    sorted streams align row-for-row. Both DBs must produce
    byte-identical output for matching rows.
    """

    name: str
    select_cols: list[str]
    query: str


def build_compares(with_stats: bool) -> list[TableCompare]:
    """Build the four per-table SELECT/ORDER queries.

    Every column appears in both SELECT and ORDER BY so the two sorted
    streams stay aligned even when a column being compared (e.g.
    `stat_common`) differs at the same natural-key position — divergent
    rows surface as ref-only + test-only entries.
    """

    # kanji: keyed by single character; serial `id` excluded.
    kanji_cols = [
        "text",
        "radical_c",
        "radical_n",
        "grade",
        "strokes",
        "freq",
    ]
    if with_stats:
        kanji_cols.extend(["stat_common", "stat_irregular"])
    kanji_query = (
        f"SELECT {', '.join(kanji_cols)} FROM kanji "
        f"ORDER BY {', '.join(c + ' COLLATE \"C\"' if c == 'text' else c for c in kanji_cols)}"
    )

    # reading: per-kanji deduped (ja_on/ja_kun → ja_onkun) + multiple
    # ja_na rows possible. Join to kanji.text for natural-key matching.
    reading_cols_select = [
        "k.text AS kanji_text",
        "r.text AS reading_text",
        "r.type",
        "r.suffixp",
        "r.prefixp",
    ]
    reading_cols_order = [
        "k.text COLLATE \"C\"",
        "r.text COLLATE \"C\"",
        "r.type COLLATE \"C\"",
        "r.suffixp",
        "r.prefixp",
    ]
    if with_stats:
        reading_cols_select.append("r.stat_common")
        reading_cols_order.append("r.stat_common")
    reading_query = (
        f"SELECT {', '.join(reading_cols_select)} "
        f"FROM reading r JOIN kanji k ON k.id = r.kanji_id "
        f"ORDER BY {', '.join(reading_cols_order)}"
    )

    # okurigana: per-reading. Compose natural key from kanji.text +
    # reading.text + reading.type + okurigana.text. Stats not on this table.
    okurigana_query = (
        "SELECT k.text AS kanji_text, r.text AS reading_text, r.type, o.text "
        "FROM okurigana o "
        "JOIN reading r ON r.id = o.reading_id "
        "JOIN kanji k ON k.id = r.kanji_id "
        "ORDER BY k.text COLLATE \"C\", r.text COLLATE \"C\", "
        "r.type COLLATE \"C\", o.text COLLATE \"C\""
    )

    # meaning: per-kanji. Compose natural key from kanji.text + meaning.text.
    meaning_query = (
        "SELECT k.text AS kanji_text, m.text "
        "FROM meaning m JOIN kanji k ON k.id = m.kanji_id "
        "ORDER BY k.text COLLATE \"C\", m.text COLLATE \"C\""
    )

    return [
        TableCompare("kanji", kanji_cols, kanji_query),
        TableCompare("reading", reading_cols_select, reading_query),
        TableCompare("okurigana",
                     ["kanji_text", "reading_text", "type", "text"],
                     okurigana_query),
        TableCompare("meaning",
                     ["kanji_text", "text"],
                     meaning_query),
    ]


def run(cmd: list[str], *, check: bool = True, **kwargs) -> subprocess.CompletedProcess:
    """Run a subprocess with helpful failure context."""
    print(f"$ {' '.join(cmd)}")
    result = subprocess.run(cmd, **kwargs)
    if check and result.returncode != 0:
        print(f"command failed (exit {result.returncode}): {' '.join(cmd)}",
              file=sys.stderr)
        sys.exit(result.returncode)
    return result


def drop_create(test_db: str) -> None:
    # `dropdb --if-exists` is non-fatal when the DB doesn't exist.
    run(["dropdb", "--if-exists", test_db])
    run(["createdb", test_db])


def apply_schema(test_db: str) -> None:
    schema_path = REPO_ROOT / "db" / "schema.sql"
    if not schema_path.exists():
        sys.exit(f"missing schema: {schema_path}")
    run(["psql", "-q", "-d", test_db, "-v", "ON_ERROR_STOP=1", "-f", str(schema_path)])


def copy_jmdict_tables(ref_db: str, test_db: str) -> None:
    """pg_dump --data-only of the three tables get_kanji_words reads,
    piped into the test DB. Required for --with-stats."""
    print(f"copying JMdict tables (entry, kanji_text, kana_text) {ref_db} → {test_db}")
    dump = subprocess.Popen(
        [
            "pg_dump",
            "--data-only",
            "--table=entry",
            "--table=kanji_text",
            "--table=kana_text",
            "-d", ref_db,
        ],
        stdout=subprocess.PIPE,
    )
    load = subprocess.Popen(
        ["psql", "-q", "-d", test_db, "-v", "ON_ERROR_STOP=1"],
        stdin=dump.stdout,
    )
    dump.stdout.close()  # type: ignore[union-attr]  # forwarded to load
    load_rc = load.wait()
    dump_rc = dump.wait()
    if dump_rc != 0 or load_rc != 0:
        sys.exit(f"jmdict table copy failed (pg_dump={dump_rc}, psql={load_rc})")


def run_loader(test_db: str, kanjidic_path: Path, with_stats: bool) -> None:
    db_url = f"postgres:///{test_db}"
    cmd = [
        "cargo", "run",
        "--release",
        "--manifest-path", str(REPO_ROOT / "Cargo.toml"),
        "-p", "kaniran-audit",
        "--bin", "e2e_load_kanjidic",
        "--",
        "--db-url", db_url,
        "--path", str(kanjidic_path),
    ]
    if with_stats:
        cmd.append("--load-stats")
    run(cmd)


# ---------------------------------------------------------------------------
# Streaming compare
# ---------------------------------------------------------------------------


def open_csv_stream(db: str, query: str) -> tuple[subprocess.Popen, Any]:
    """Stream a sorted CSV from psql. Memory: bounded by line buffer."""
    cmd = [
        "psql",
        "-d", db,
        "-v", "ON_ERROR_STOP=1",
        "-c", rf"\COPY ({query}) TO STDOUT WITH (FORMAT csv)",
    ]
    proc = subprocess.Popen(
        cmd, stdout=subprocess.PIPE, text=True, bufsize=1,
        # Python's csv reader expects no embedded \r\n inside fields;
        # psql writes \n line terminators by default in CSV mode.
    )
    reader = csv.reader(proc.stdout)  # type: ignore[arg-type]
    return proc, reader


def _next(it: Any) -> Optional[list[str]]:
    """`next()` that returns None instead of raising StopIteration."""
    try:
        return next(it)
    except StopIteration:
        return None


def compare_table(tc: TableCompare, ref_db: str, test_db: str,
                  report) -> tuple[int, int, int]:
    """Stream-merge two sorted CSV outputs. Returns (matches, ref_only, test_only).

    Both psql processes stream; the merge holds at most one row from
    each side at any time. Every mismatch is written to the report file
    as it's seen (no buffering across the whole table)."""
    ref_proc, ref_iter = open_csv_stream(ref_db, tc.query)
    test_proc, test_iter = open_csv_stream(test_db, tc.query)

    report.write(f"\n## {tc.name}\n")
    report.write(f"columns: {', '.join(tc.select_cols)}\n\n")

    matches = 0
    ref_only = 0
    test_only = 0
    ref_row = _next(ref_iter)
    test_row = _next(test_iter)
    while ref_row is not None and test_row is not None:
        if ref_row == test_row:
            matches += 1
            ref_row = _next(ref_iter)
            test_row = _next(test_iter)
        elif ref_row < test_row:
            ref_only += 1
            report.write(f"REF ONLY  {ref_row}\n")
            ref_row = _next(ref_iter)
        else:
            test_only += 1
            report.write(f"TEST ONLY {test_row}\n")
            test_row = _next(test_iter)
    while ref_row is not None:
        ref_only += 1
        report.write(f"REF ONLY  {ref_row}\n")
        ref_row = _next(ref_iter)
    while test_row is not None:
        test_only += 1
        report.write(f"TEST ONLY {test_row}\n")
        test_row = _next(test_iter)

    ref_proc.wait()
    test_proc.wait()
    report.write(
        f"\nsummary: matches={matches} ref_only={ref_only} test_only={test_only}\n"
    )
    return matches, ref_only, test_only


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--ref-db", default="ichiran_260118",
                    help="reference DB (default: ichiran_260118 — restored "
                         "from ichiran-260118.pgdump per the module docstring)")
    ap.add_argument("--test-db", default="ichiran_kanji_e2e",
                    help="test DB (default: ichiran_kanji_e2e)")
    ap.add_argument("--kanjidic-path",
                    default=str(REPO_ROOT / "bookkeeping" / "e2e" / "fixtures"
                                / "kanjidic2_2015-03-17.xml"),
                    help="path to kanjidic2.xml (default: the 2015-03-17 "
                         "Wayback snapshot — empirically reproduces ichiran-260118)")
    ap.add_argument("--with-stats", action="store_true",
                    help="copy JMdict tables and run load_kanji_stats too")
    ap.add_argument("--skip-drop", action="store_true",
                    help="skip dropdb/createdb; reuse existing test DB shell")
    ap.add_argument("--skip-load", action="store_true",
                    help="skip the load step; compare current test-DB state")
    ap.add_argument("--report",
                    default="bookkeeping/e2e/last_run.md",
                    help="report path (relative to repo root)")
    args = ap.parse_args()

    kanjidic_path = Path(args.kanjidic_path).resolve()
    if not args.skip_load and not kanjidic_path.exists():
        sys.exit(f"kanjidic file not found: {kanjidic_path}")
    report_path = REPO_ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)

    t0 = time.time()
    if not args.skip_load:
        if not args.skip_drop:
            drop_create(args.test_db)
            apply_schema(args.test_db)
        if args.with_stats:
            copy_jmdict_tables(args.ref_db, args.test_db)
        run_loader(args.test_db, kanjidic_path, args.with_stats)
    t_load = time.time() - t0

    compares = build_compares(args.with_stats)
    totals = {"matches": 0, "ref_only": 0, "test_only": 0}
    with open(report_path, "w") as report:
        report.write(f"# kanji_loader_e2e — {args.ref_db} vs {args.test_db}\n\n")
        report.write(f"with_stats: {args.with_stats}\n")
        report.write(f"kanjidic: {kanjidic_path}\n")
        report.write(f"load duration: {t_load:.1f}s\n")
        for tc in compares:
            m, r, t = compare_table(tc, args.ref_db, args.test_db, report)
            totals["matches"] += m
            totals["ref_only"] += r
            totals["test_only"] += t
            print(f"  {tc.name}: matches={m} ref_only={r} test_only={t}")
        report.write(
            f"\n# overall\nmatches={totals['matches']} "
            f"ref_only={totals['ref_only']} test_only={totals['test_only']}\n"
        )

    print(f"\nreport: {report_path}")
    print(f"overall matches={totals['matches']} "
          f"ref_only={totals['ref_only']} test_only={totals['test_only']}")
    return 0 if totals["ref_only"] == 0 and totals["test_only"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())

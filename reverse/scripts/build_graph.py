#!/usr/bin/env python3
"""
Parse the auto-generated *.md files under reverse/ into two normalized CSVs:

  symbols.csv  fqn, name, package, file, line, kind, status
  edges.csv    caller_fqn, callee_fqn, resolved

Run from repo root:
    python3 reverse/scripts/build_graph.py
"""
from __future__ import annotations

import csv
import os
import re
import sys
from pathlib import Path

REVERSE_DIR = Path(__file__).resolve().parent.parent
OUT_DIR = REVERSE_DIR / "scripts"
SYMBOLS_CSV = OUT_DIR / "symbols.csv"
EDGES_CSV = OUT_DIR / "edges.csv"

H1 = re.compile(r"^#\s+(\S.*?)\s*$")
PACKAGE = re.compile(r"^\*\*Package:\*\*\s+`([^`]+)`")
SOURCE = re.compile(r"^\*\*Source:\*\*\s+`([^`:]+):(\d+)`")
DEFFORM = re.compile(r"^\*\*Definition form:\*\*\s+`([^`]+)`")
DEP = re.compile(r"^- `([^`]+)`")
DEP_HEADER = re.compile(r"^##\s+Dependencies")

KIND_MAP = {"defun": "fn", "defmacro": "macro", "defgeneric": "gf"}


def fqn(package: str, name: str) -> str:
    return f"{package}:{name}"


def split_dep(token: str) -> tuple[str, str]:
    """`ichiran/characters:test-word` or `ichiran::strings` → (package, name)."""
    # collapse `::` (internal) to `:` (single separator)
    if "::" in token:
        pkg, _, name = token.partition("::")
    else:
        pkg, _, name = token.partition(":")
    return pkg, name


def parse_md(path: Path) -> tuple[dict, list[str]] | None:
    name = package = source_file = kind = None
    line_no = 0
    deps: list[str] = []
    in_deps = False

    with path.open(encoding="utf-8") as fh:
        for raw in fh:
            line = raw.rstrip("\n")
            if name is None:
                m = H1.match(line)
                if m:
                    name = m.group(1).strip()
                    continue
            if package is None:
                m = PACKAGE.match(line)
                if m:
                    package = m.group(1)
                    continue
            if source_file is None:
                m = SOURCE.match(line)
                if m:
                    source_file = m.group(1)
                    line_no = int(m.group(2))
                    continue
            if kind is None:
                m = DEFFORM.match(line)
                if m:
                    kind = KIND_MAP.get(m.group(1), m.group(1))
                    continue
            if DEP_HEADER.match(line):
                in_deps = True
                continue
            if in_deps:
                if line.startswith("## "):  # next section
                    in_deps = False
                    continue
                m = DEP.match(line)
                if m:
                    deps.append(m.group(1))

    if not (name and package):
        return None

    return (
        {
            "fqn": fqn(package, name),
            "name": name,
            "package": package,
            "file": source_file or "",
            "line": line_no,
            "kind": kind or "",
            "status": "pending",
        },
        deps,
    )


def main() -> int:
    md_files = sorted(REVERSE_DIR.glob("*.lisp/*.md"))
    if not md_files:
        print(f"no md files under {REVERSE_DIR}", file=sys.stderr)
        return 1

    rows: list[dict] = []
    pending_edges: list[tuple[str, str]] = []

    for path in md_files:
        parsed = parse_md(path)
        if parsed is None:
            print(f"skip (no header): {path}", file=sys.stderr)
            continue
        sym, deps = parsed
        rows.append(sym)
        for dep in deps:
            pkg, name = split_dep(dep)
            pending_edges.append((sym["fqn"], fqn(pkg, name)))

    by_fqn = {r["fqn"]: r for r in rows}
    edges: list[tuple[str, str, int]] = []
    unresolved: set[str] = set()
    for caller, callee in pending_edges:
        resolved = 1 if callee in by_fqn else 0
        if not resolved:
            unresolved.add(callee)
        edges.append((caller, callee, resolved))

    OUT_DIR.mkdir(parents=True, exist_ok=True)

    with SYMBOLS_CSV.open("w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(
            fh,
            fieldnames=["fqn", "name", "package", "file", "line", "kind", "status"],
        )
        w.writeheader()
        for r in sorted(rows, key=lambda r: r["fqn"]):
            w.writerow(r)

    with EDGES_CSV.open("w", newline="", encoding="utf-8") as fh:
        w = csv.writer(fh)
        w.writerow(["caller_fqn", "callee_fqn", "resolved"])
        for row in sorted(set(edges)):
            w.writerow(row)

    rel = lambda p: os.path.relpath(p, REVERSE_DIR.parent)
    print(f"symbols: {len(rows)} -> {rel(SYMBOLS_CSV)}")
    print(f"edges:   {len(edges)} -> {rel(EDGES_CSV)}")
    if unresolved:
        print(f"unresolved callees: {len(unresolved)} (external / built-in)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

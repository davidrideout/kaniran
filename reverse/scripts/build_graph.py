#!/usr/bin/env python3
"""
Parse the auto-generated *.md files under reverse/ into two normalized CSVs:

  symbols.csv  fqn, name, package, file, line, kind, status, reason
  edges.csv    caller_fqn, callee_fqn, resolved, origin

The `reason` column captures *why* a symbol is off-the-books (e.g. why a
`skip` was chosen over `pending`). Free-form text; empty for pending /
ported rows. build_graph.py wipes both `status` and `reason` on every
run — commit symbols.csv first or re-record via `query.py mark --reason`.

The `origin` column on edges.csv records which extraction pass found the
edge: `runtime` (SBCL who-calls — function-call edges only), `source`
(introspector's source-walk pass — catches data references like quoted
DAO-class symbols passed to postmodern helpers), or `both`.

Recognised md kinds (by filename suffix):
  *.md            function / macro / generic   — dependency = "## Dependencies"
                                                 + "## Source-walked references"
  *_struct.md     defstruct                    — dependency = :include parent struct
  *_class.md      defclass                     — dependency = direct superclasses
  *_dao.md        defclass :metaclass dao-class — dependency = direct superclasses
  *_global.md     defparameter / defvar / defconstant
                                               — dependency = symbol references
                                                 inside the printed value block
  *_type.md       deftype                      — dependency = members referenced
  *_condition.md  define-condition             — dependency = inherited condition

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

# H1 forms:
#   "# kr-canonical"
#   "# kana-representation (defstruct)"
#   "# *sokuon-characters* (global variable)"
H1 = re.compile(r"^#\s+(\S.*?)(?:\s+\([^)]+\))?\s*$")
PACKAGE = re.compile(r"^\*\*Package:\*\*\s+`([^`]+)`")
SOURCE = re.compile(r"^\*\*Source:\*\*\s+`([^`:]+)(?::(\d+))?`")
DEFFORM = re.compile(r"^\*\*Definition form:\*\*\s+`([^`]+)`")
DEP = re.compile(r"^- `([^`]+)`")
DEP_HEADER = re.compile(r"^##\s+Dependencies")
SOURCE_WALK_HEADER = re.compile(r"^##\s+Source-walked references")

INCLUDE = re.compile(r"^\*\*Include:\*\*\s+`([^`]+)`")
INHERITS = re.compile(r"^\*\*Inherits:\*\*\s+`([^`]+)`")
SUPERS = re.compile(r"^- Direct supers:\s+(.+)$")
TICK = re.compile(r"`([^`]+)`")
VALUE_FENCE = re.compile(r"^```")
# Symbol mention inside a value block, e.g. ICHIRAN/DICT::ABBR-II
SYMBOL_REF = re.compile(r"ICHIRAN(?:/[A-Z0-9-]+)?::?[A-Z0-9*+/_-]+")

KIND_MAP = {"defun": "fn", "defmacro": "macro", "defgeneric": "gf"}

# Suffix → kind label written into symbols.csv. Order matters for
# longest-match (none collide here, but keep explicit for clarity).
SUFFIX_KIND = [
    ("_struct.md", "struct"),
    ("_class.md", "class"),
    ("_dao.md", "dao"),
    ("_global.md", "global"),
    ("_type.md", "type"),
    ("_condition.md", "condition"),
]


def fqn(package: str, name: str) -> str:
    return f"{package}:{name}"


def split_dep(token: str) -> tuple[str, str]:
    """`ichiran/characters:test-word` or `ichiran::strings` → (package, name)."""
    if "::" in token:
        pkg, _, name = token.partition("::")
    else:
        pkg, _, name = token.partition(":")
    return pkg, name


def normalise_dep(token: str) -> str:
    """Lowercase the package and name for consistent CSV joining."""
    pkg, name = split_dep(token.strip("`"))
    return fqn(pkg.lower(), name.lower())


def aux_kind(path: Path) -> str | None:
    name = path.name
    for suffix, kind in SUFFIX_KIND:
        if name.endswith(suffix):
            return kind
    return None


def parse_value_block_deps(lines: list[str], start_idx: int) -> list[str]:
    """From a ```lisp fenced block, extract every ICHIRAN-package symbol
    reference inside. Returns lowercased FQNs."""
    deps: list[str] = []
    i = start_idx + 1
    while i < len(lines):
        if VALUE_FENCE.match(lines[i]):
            break
        for m in SYMBOL_REF.finditer(lines[i]):
            deps.append(normalise_dep(m.group(0)))
        i += 1
    return deps


def parse_md(path: Path) -> tuple[dict, list[str], list[str]] | None:
    """Returns ({symbol-row}, [runtime-deps], [source-walk-deps]) or None."""
    raw_lines = path.read_text(encoding="utf-8").splitlines()
    name = package = source_file = kind_field = None
    line_no = 0
    deps: list[str] = []
    source_walk_deps: list[str] = []
    in_deps = False

    suffix_kind = aux_kind(path)

    # First pass: header fields.
    for idx, line in enumerate(raw_lines):
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
                line_no = int(m.group(2)) if m.group(2) else 0
                continue
        if kind_field is None and suffix_kind is None:
            m = DEFFORM.match(line)
            if m:
                kind_field = KIND_MAP.get(m.group(1), m.group(1))
                continue

    if not (name and package):
        return None

    # Second pass: dependency extraction (kind-specific).
    if suffix_kind is None:
        # Functions / macros / generics: two sections.
        # `## Dependencies` is the runtime call-graph (precise per-method when
        # nested under `### method (...)`).
        # `## Source-walked references` is the source-walk pass — catches DAO
        # class symbols and other data references invisible to the call graph.
        in_source_walk = False
        for line in raw_lines:
            if DEP_HEADER.match(line):
                in_deps, in_source_walk = True, False
                continue
            if SOURCE_WALK_HEADER.match(line):
                in_deps, in_source_walk = False, True
                continue
            if in_deps or in_source_walk:
                if line.startswith("## "):
                    in_deps = in_source_walk = False
                    continue
                m = DEP.match(line)
                if m:
                    target = source_walk_deps if in_source_walk else deps
                    target.append(normalise_dep(m.group(1)))
        kind_out = kind_field or ""
    elif suffix_kind in ("struct",):
        for line in raw_lines:
            m = INCLUDE.match(line)
            if m and m.group(1).strip().upper() != "NIL":
                # Bare struct name, e.g. `SOMETHING` — assume same package.
                inc = m.group(1).strip()
                deps.append(normalise_dep(f"{package}:{inc}"))
                break
        kind_out = "struct"
    elif suffix_kind in ("class", "dao"):
        for line in raw_lines:
            m = SUPERS.match(line)
            if m:
                # Pull each backticked symbol; superclasses can be
                # bare (same-package) or fully-qualified.
                for tok in TICK.findall(m.group(1)):
                    if tok.upper() == "STANDARD-OBJECT" or tok.lower() == "_(none)_":
                        continue
                    if ":" in tok:
                        deps.append(normalise_dep(tok))
                    else:
                        deps.append(normalise_dep(f"{package}:{tok}"))
                break
        kind_out = suffix_kind
    elif suffix_kind == "global":
        # Find the ```lisp fence and walk for symbol references.
        for idx, line in enumerate(raw_lines):
            if VALUE_FENCE.match(line) and "lisp" in line:
                deps.extend(parse_value_block_deps(raw_lines, idx))
                break
        kind_out = "global"
    elif suffix_kind == "type":
        # The single deftype in the codebase has no ichiran-symbol deps;
        # walking the value block harmlessly returns nothing.
        for idx, line in enumerate(raw_lines):
            if VALUE_FENCE.match(line) and "lisp" in line:
                deps.extend(parse_value_block_deps(raw_lines, idx))
                break
        kind_out = "type"
    elif suffix_kind == "condition":
        for line in raw_lines:
            m = INHERITS.match(line)
            if m and m.group(1).upper() != "ERROR":
                deps.append(normalise_dep(f"{package}:{m.group(1).strip()}"))
                break
        kind_out = "condition"
    else:
        kind_out = suffix_kind

    return (
        {
            "fqn": fqn(package, name),
            "name": name,
            "package": package,
            "file": source_file or "",
            "line": line_no,
            "kind": kind_out,
            "status": "pending",
            "reason": "",
        },
        deps,
        source_walk_deps,
    )


def main() -> int:
    md_files = sorted(REVERSE_DIR.glob("*.lisp/*.md"))
    if not md_files:
        print(f"no md files under {REVERSE_DIR}", file=sys.stderr)
        return 1

    rows: list[dict] = []
    runtime_pending: list[tuple[str, str]] = []
    source_pending: list[tuple[str, str]] = []
    seen: set[str] = set()

    for path in md_files:
        parsed = parse_md(path)
        if parsed is None:
            print(f"skip (no header): {path}", file=sys.stderr)
            continue
        sym, deps, swdeps = parsed
        # If the same fqn appears via both a function md and a struct md
        # (e.g. autogenerated accessor + struct itself collide), keep the
        # first one encountered alphabetically and warn. In practice the
        # struct/class fqns are distinct from their accessor fqns so this
        # only fires on real duplicates.
        if sym["fqn"] in seen:
            print(
                f"duplicate fqn: {sym['fqn']} (skipping {path.name})",
                file=sys.stderr,
            )
            continue
        seen.add(sym["fqn"])
        rows.append(sym)
        for dep in deps:
            runtime_pending.append((sym["fqn"], dep))
        for dep in swdeps:
            source_pending.append((sym["fqn"], dep))

    by_fqn = {r["fqn"]: r for r in rows}

    # Source-walk over-collects (locals, lambda-list params, slot names).
    # Drop callees that aren't a known top-level definition. Self-edges
    # (introspector also filters them but belt-and-suspenders) get dropped.
    source_filtered: list[tuple[str, str]] = []
    source_dropped = 0
    for caller, callee in source_pending:
        if callee not in by_fqn or callee == caller:
            source_dropped += 1
            continue
        source_filtered.append((caller, callee))

    # Union with origin tagging.
    runtime_set = set(runtime_pending)
    source_set = set(source_filtered)
    edges: list[tuple[str, str, int, str]] = []
    unresolved: set[str] = set()
    seen_pairs: set[tuple[str, str]] = set()
    runtime_only = 0
    source_only = 0
    both_count = 0
    for caller, callee in sorted(runtime_set | source_set):
        if (caller, callee) in seen_pairs:
            continue
        seen_pairs.add((caller, callee))
        in_runtime = (caller, callee) in runtime_set
        in_source = (caller, callee) in source_set
        if in_runtime and in_source:
            origin = "both"
            both_count += 1
        elif in_source:
            origin = "source"
            source_only += 1
        else:
            origin = "runtime"
            runtime_only += 1
        resolved = 1 if callee in by_fqn else 0
        if not resolved:
            unresolved.add(callee)
        edges.append((caller, callee, resolved, origin))

    OUT_DIR.mkdir(parents=True, exist_ok=True)

    with SYMBOLS_CSV.open("w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(
            fh,
            fieldnames=["fqn", "name", "package", "file", "line", "kind", "status", "reason"],
        )
        w.writeheader()
        for r in sorted(rows, key=lambda r: r["fqn"]):
            w.writerow(r)

    with EDGES_CSV.open("w", newline="", encoding="utf-8") as fh:
        w = csv.writer(fh)
        w.writerow(["caller_fqn", "callee_fqn", "resolved", "origin"])
        for row in sorted(edges):
            w.writerow(row)

    rel = lambda p: os.path.relpath(p, REVERSE_DIR.parent)
    print(f"symbols: {len(rows)} -> {rel(SYMBOLS_CSV)}")
    print(f"edges:   {len(edges)} -> {rel(EDGES_CSV)}")
    print(
        f"  origin: runtime={runtime_only}, source={source_only}, both={both_count} "
        f"(source-walk dropped {source_dropped} non-top-level / self refs)"
    )
    if unresolved:
        print(f"unresolved callees: {len(unresolved)} (external / built-in)")

    # Per-kind breakdown for sanity.
    from collections import Counter
    kinds = Counter(r["kind"] for r in rows)
    print(
        "by kind: "
        + ", ".join(f"{k}={n}" for k, n in sorted(kinds.items(), key=lambda x: -x[1]))
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

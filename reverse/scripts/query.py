#!/usr/bin/env python3
"""
Query the dependency graph emitted by build_graph.py.

Usage:
    query.py leaves                  # symbols with no resolved callees
    query.py next                    # symbols whose every resolved callee has status='ported'
    query.py dependents <fqn>        # callers of <fqn>  (transitive: --deep)
    query.py deps <fqn>              # callees of <fqn>  (transitive: --deep)
    query.py layers                  # assign topological layer to every symbol
    query.py mark <fqn>... --status ported [--reason "text"]
    query.py stats
    query.py plan [--out FILE] [--skip-packages PKG,PKG] [--include-status STATUS]

<fqn> may be a full FQN (`ichiran:kr-branch`) or a bare name (`kr-branch`)
when unambiguous.

Unresolved callees (external / built-in) are ignored when computing leaves
and the next layer — those calls don't block porting.
"""
from __future__ import annotations

import argparse
import csv
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Iterator

HERE = Path(__file__).resolve().parent
SYMBOLS_CSV = HERE / "symbols.csv"
EDGES_CSV = HERE / "edges.csv"
SIGNATURES_JSON = HERE / "signatures.json"
DIVERGENCES_MD = HERE / "divergences.md"
KANIRAN_CORE_SRC = HERE.parent.parent / "kaniran-core" / "src"


def load() -> tuple[dict[str, dict], dict[str, set[str]], dict[str, set[str]]]:
    """Returns (symbols_by_fqn, callees[fqn]→fqns, callers[fqn]→fqns).

    Only resolved=1 edges are followed for graph traversal.
    """
    if not SYMBOLS_CSV.exists() or not EDGES_CSV.exists():
        sys.exit(f"missing CSVs — run build_graph.py first ({HERE})")

    syms: dict[str, dict] = {}
    with SYMBOLS_CSV.open(encoding="utf-8") as fh:
        for r in csv.DictReader(fh):
            syms[r["fqn"]] = r

    callees: dict[str, set[str]] = defaultdict(set)
    callers: dict[str, set[str]] = defaultdict(set)
    with EDGES_CSV.open(encoding="utf-8") as fh:
        for r in csv.DictReader(fh):
            if r["resolved"] != "1":
                continue
            caller, callee = r["caller_fqn"], r["callee_fqn"]
            callees[caller].add(callee)
            callers[callee].add(caller)
    return syms, callees, callers


def resolve_fqn(token: str, syms: dict[str, dict]) -> str:
    if token in syms:
        return token
    matches = [f for f in syms if f.split(":", 1)[1] == token]
    if len(matches) == 1:
        return matches[0]
    if not matches:
        sys.exit(f"unknown symbol: {token}")
    sys.exit(f"ambiguous symbol {token!r}, candidates:\n  " + "\n  ".join(matches))


def fmt(sym: dict) -> str:
    reason = sym.get("reason", "")
    suffix = f" — {reason}" if reason else ""
    tags = _workstream_tags(sym, sep=" ")
    if tags:
        tags = f"  {tags}"
    return (
        f"{sym['fqn']:<55} {sym['kind']:<5} {sym['file']}:{sym['line']}  "
        f"[{sym['status']}{suffix}]{tags}"
    )


def _badge(sym: dict) -> str:
    """Plan-line badge for non-pending symbols. Shows the reason when present
    so a reader of PORT_PLAN.md can see why something is off the books."""
    reason = sym.get("reason", "")
    if reason:
        return f"  *[{sym['status']} — {reason}]*"
    return f"  *[{sym['status']}]*"


def _workstream_tags(sym: dict, sep: str = "") -> str:
    """Render the `extracted` / `audited` parallel-workstream columns
    as inline badges. Empty string when neither is set, so the caller
    can append unconditionally without worrying about trailing space."""
    parts: list[str] = []
    extracted = sym.get("extracted", "") or ""
    audited = sym.get("audited", "") or ""
    if extracted:
        parts.append(f"*[extracted: {extracted}]*")
    if audited:
        parts.append(f"*[audited {audited}]*")
    return sep.join(parts) if sep else "  ".join(parts)


def cmd_leaves(args, syms, callees, callers):
    rows = [
        s for fqn, s in syms.items()
        if s["status"] == "pending" and not callees.get(fqn)
    ]
    rows.sort(key=lambda s: (s["package"], s["name"]))
    for s in rows:
        print(fmt(s))
    print(f"\n{len(rows)} pending leaf symbol(s)", file=sys.stderr)


def cmd_next(args, syms, callees, callers):
    """Symbols whose every resolved callee is already ported."""
    out = []
    for fqn, s in syms.items():
        if s["status"] != "pending":
            continue
        cs = callees.get(fqn, set())
        if not cs:
            continue  # leaves — use `leaves` cmd
        if all(syms[c]["status"] == "ported" for c in cs):
            out.append(s)
    out.sort(key=lambda s: (s["package"], s["name"]))
    for s in out:
        print(fmt(s))
    print(f"\n{len(out)} symbol(s) ready for next wave", file=sys.stderr)


def _walk(start: str, edges: dict[str, set[str]], deep: bool) -> set[str]:
    if not deep:
        return set(edges.get(start, ()))
    seen: set[str] = set()
    stack = [start]
    while stack:
        cur = stack.pop()
        for nxt in edges.get(cur, ()):
            if nxt not in seen:
                seen.add(nxt)
                stack.append(nxt)
    return seen


def cmd_dependents(args, syms, callees, callers):
    target = resolve_fqn(args.fqn, syms)
    fqns = _walk(target, callers, args.deep)
    for f in sorted(fqns):
        print(fmt(syms[f]))
    print(f"\n{len(fqns)} dependent(s) of {target}", file=sys.stderr)


def cmd_deps(args, syms, callees, callers):
    target = resolve_fqn(args.fqn, syms)
    fqns = _walk(target, callees, args.deep)
    for f in sorted(fqns):
        print(fmt(syms[f]))
    print(f"\n{len(fqns)} dependenc{'ies' if len(fqns)!=1 else 'y'} of {target}", file=sys.stderr)


def cmd_layers(args, syms, callees, callers):
    """Topological layer per symbol: layer = 1 + max(layer of resolved callees), 0 for leaves.
    Cycles get layer = -1."""
    layer: dict[str, int] = {}
    visiting: set[str] = set()

    def visit(f: str) -> int:
        if f in layer:
            return layer[f]
        if f in visiting:
            layer[f] = -1
            return -1
        visiting.add(f)
        cs = callees.get(f, set())
        if not cs:
            layer[f] = 0
        else:
            child_layers = [visit(c) for c in cs]
            if -1 in child_layers:
                layer[f] = -1
            else:
                layer[f] = 1 + max(child_layers)
        visiting.discard(f)
        return layer[f]

    for f in syms:
        visit(f)

    by_layer: dict[int, list[str]] = defaultdict(list)
    for f, lv in layer.items():
        by_layer[lv].append(f)
    for lv in sorted(by_layer):
        label = "CYCLE" if lv == -1 else f"L{lv}"
        print(f"== {label}  ({len(by_layer[lv])}) ==")
        for f in sorted(by_layer[lv]):
            print(f"  {fmt(syms[f])}")


SYMBOL_FIELDS = [
    "fqn", "name", "package", "file", "line", "kind",
    "status", "reason", "extracted", "audited",
]


def _write_symbols(syms: dict[str, dict]) -> None:
    """Round-trip-safe write of `syms` back to symbols.csv. Ensures the
    full field set is present on every row so older CSVs from before the
    `extracted`/`audited` split don't drop columns."""
    with SYMBOLS_CSV.open("w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=SYMBOL_FIELDS)
        w.writeheader()
        for r in sorted(syms.values(), key=lambda r: r["fqn"]):
            for field in SYMBOL_FIELDS:
                r.setdefault(field, "")
            w.writerow(r)


def cmd_mark(args, syms, callees, callers):
    targets = [resolve_fqn(t, syms) for t in args.fqns]
    for t in targets:
        syms[t]["status"] = args.status
        if args.reason is not None:
            syms[t]["reason"] = args.reason
    _write_symbols(syms)
    detail = f" with reason {args.reason!r}" if args.reason is not None else ""
    print(f"marked {len(targets)} symbol(s) as {args.status}{detail}", file=sys.stderr)


def cmd_extracted(args, syms, callees, callers):
    targets = [resolve_fqn(t, syms) for t in args.fqns]
    if args.reset:
        new_value = ""
    else:
        if not args.corpus:
            sys.exit("--corpus required (e.g. tatoeba, init-suffixes); pass --reset to clear")
        new_value = args.corpus
    for t in targets:
        syms[t]["extracted"] = new_value
    _write_symbols(syms)
    label = "cleared" if args.reset else f"tagged as extracted={new_value!r}"
    print(f"{len(targets)} symbol(s) {label}", file=sys.stderr)


def cmd_audited(args, syms, callees, callers):
    targets = [resolve_fqn(t, syms) for t in args.fqns]
    if args.reset:
        new_value = ""
    else:
        if args.pass_count is None or args.total is None:
            sys.exit("--pass and --total required (or --reset to clear)")
        for t in targets:
            extracted = syms[t].get("extracted", "") or ""
            if not extracted:
                sys.exit(
                    f"{t}: audited requires a prior extraction; current extracted is empty. "
                    f"Run `query.py extracted {t} --corpus <tag>` first."
                )
        if args.pass_count > args.total:
            sys.exit(f"--pass ({args.pass_count}) cannot exceed --total ({args.total})")
        if args.pass_count < args.total:
            fail = args.total - args.pass_count
            new_value = f"{args.pass_count}/{args.total} ({fail} fail)"
        else:
            new_value = f"{args.pass_count}/{args.total}"
    for t in targets:
        syms[t]["audited"] = new_value
    _write_symbols(syms)
    label = "cleared" if args.reset else f"tagged as audited={new_value!r}"
    print(f"{len(targets)} symbol(s) {label}", file=sys.stderr)


def cmd_stats(args, syms, callees, callers):
    by_status: dict[str, int] = defaultdict(int)
    by_pkg_status: dict[tuple[str, str], int] = defaultdict(int)
    for s in syms.values():
        by_status[s["status"]] += 1
        by_pkg_status[(s["package"], s["status"])] += 1
    print("== overall ==")
    for k in sorted(by_status):
        print(f"  {k:<10} {by_status[k]}")
    print("\n== by package ==")
    pkgs = sorted({p for p, _ in by_pkg_status})
    statuses = sorted({s for _, s in by_pkg_status})
    print(f"  {'package':<25} " + " ".join(f"{s:>8}" for s in statuses))
    for p in pkgs:
        cells = " ".join(f"{by_pkg_status[(p,s)]:>8}" for s in statuses)
        print(f"  {p:<25} {cells}")


def _tarjan_sccs(nodes: list[str], callees: dict[str, set[str]]) -> list[list[str]]:
    """Tarjan's strongly-connected-components. Returns SCCs in reverse-topo order
    (leaves first), which is exactly the order we want for a port plan."""
    index_of: dict[str, int] = {}
    lowlink: dict[str, int] = {}
    on_stack: set[str] = set()
    stack: list[str] = []
    sccs: list[list[str]] = []
    counter = [0]

    def strongconnect(v: str) -> None:
        # iterative version to avoid Python recursion limits on the big dict.lisp blob
        work: list[tuple[str, Iterator[str]]] = [(v, iter(sorted(callees.get(v, ()))))]
        index_of[v] = lowlink[v] = counter[0]
        counter[0] += 1
        stack.append(v); on_stack.add(v)
        while work:
            node, it = work[-1]
            try:
                w = next(it)
            except StopIteration:
                work.pop()
                if lowlink[node] == index_of[node]:
                    comp = []
                    while True:
                        x = stack.pop(); on_stack.discard(x); comp.append(x)
                        if x == node:
                            break
                    sccs.append(comp)
                if work:
                    parent, _ = work[-1]
                    if lowlink[node] < lowlink[parent]:
                        lowlink[parent] = lowlink[node]
                continue
            if w not in index_of:
                index_of[w] = lowlink[w] = counter[0]
                counter[0] += 1
                stack.append(w); on_stack.add(w)
                work.append((w, iter(sorted(callees.get(w, ())))))
            elif w in on_stack:
                if index_of[w] < lowlink[node]:
                    lowlink[node] = index_of[w]

    for n in nodes:
        if n not in index_of:
            strongconnect(n)
    return sccs


def cmd_plan(args, syms, callees, callers):
    skip = set(args.skip_packages.split(",")) if args.skip_packages else set()
    include = set(args.include_status.split(",")) if args.include_status else None

    nodes = [
        f for f, s in syms.items()
        if s["package"] not in skip
        and (include is None or s["status"] in include)
    ]
    node_set = set(nodes)
    # restrict edges to the subgraph
    sub_callees = {n: {c for c in callees.get(n, ()) if c in node_set} for n in nodes}

    sccs = _tarjan_sccs(nodes, sub_callees)
    # Tarjan emits SCCs in reverse-topo order, which is the order we want.
    scc_of = {n: i for i, comp in enumerate(sccs) for n in comp}

    out_lines: list[str] = []
    cycle_count = sum(1 for c in sccs if len(c) > 1)
    cycle_nodes = sum(len(c) for c in sccs if len(c) > 1)
    out_lines.append(f"# Port plan — {len(nodes)} symbols in {len(sccs)} waves "
                     f"({cycle_count} mutual-recursion groups covering {cycle_nodes} symbols)")
    if skip:
        out_lines.append(f"_skipped packages: {', '.join(sorted(skip))}_")
    out_lines.append("")

    def render_row(s: dict) -> str:
        badge = "" if s["status"] == "pending" else _badge(s)
        tags = _workstream_tags(s)
        if tags:
            tags = f"  {tags}"
        return f"`{s['fqn']}`  — {s['kind']}, {s['file']}:{s['line']}{badge}{tags}"

    for i, comp in enumerate(sccs):
        members = sorted(comp)
        if len(members) == 1:
            s = syms[members[0]]
            out_lines.append(f"{i+1:>4}. {render_row(s)}")
        else:
            out_lines.append(
                f"{i+1:>4}. **CYCLE ({len(members)} symbols — port together)**"
            )
            for fqn in members:
                out_lines.append(f"        - {render_row(syms[fqn])}")

    text = "\n".join(out_lines) + "\n"
    if args.out:
        Path(args.out).write_text(text, encoding="utf-8")
        print(f"wrote {args.out} ({len(sccs)} waves, {len(nodes)} symbols)", file=sys.stderr)
    else:
        sys.stdout.write(text)


# ─── audit-signatures ────────────────────────────────────────────────────────
#
# Cross-reference every ported callable (fn / macro / gf) against signatures.json
# to flag arity drift, missing pub fns, and extra public surface (the failure
# mode that produced the `_with` split during the numbers port).
#
# Lisp arity = required + optional + len(keys); &rest is ignored (it usually
# represents an apply-forwarding artifact rather than a user-visible param).
# Rust arity = top-level comma-separated args in the `pub fn name(...)` body.
#
# Limits of the audit:
# - Doesn't check parameter *types*, only count.
# - Macros frequently port to doc-only files (CONVENTIONS §4.8); audit skips
#   pub fn checks for them and only verifies file existence.
# - Lambda lists with malformed defaults will degrade gracefully (key count
#   may be off) — the report flags those as `lambda-parse-fallback`.

KIND_SUFFIX = {
    "fn": "", "gf": "", "global": "",
    "macro": "_macro", "struct": "_struct", "class": "_class",
    "dao": "_dao", "type": "_type", "condition": "_condition",
}


def _translate_chars(s: str) -> str:
    out: list[str] = []
    for c in s:
        if c == "*":
            out.append("_star_")
        elif c == "+":
            out.append("_plus_")
        elif c == "-":
            out.append("_")
        else:
            out.append(c)
    return _collapse_underscores("".join(out))


def _collapse_underscores(s: str) -> str:
    out: list[str] = []
    prev = False
    for c in s:
        if c == "_":
            if not prev:
                out.append(c)
            prev = True
        else:
            out.append(c)
            prev = False
    return "".join(out)


def fqn_to_rust_path(fqn: str, kind: str) -> str:
    """Python mirror of `kani::naming::fqn_to_path` — see naming.rs for the spec."""
    pkg, _, name = fqn.partition(":")
    if not name:
        raise ValueError(f"empty name in FQN: {fqn}")
    pkg_lower = pkg.lower()
    if pkg_lower.startswith("ichiran/"):
        module_dir = _translate_chars(pkg_lower[len("ichiran/"):])
    elif pkg_lower == "ichiran":
        module_dir = "core"
    else:
        module_dir = _translate_chars(pkg_lower)
    file_stem = _translate_chars(name.lower())
    suffix = KIND_SUFFIX.get(kind, "")
    if suffix:
        file_stem = _collapse_underscores(file_stem + suffix)
    return f"{module_dir}/{file_stem}.rs"


def _split_top_level_lisp(s: str) -> list[str]:
    """Split an s-expr body by whitespace, respecting paren nesting and strings."""
    tokens: list[str] = []
    cur: list[str] = []
    depth = 0
    in_str = False
    i = 0
    while i < len(s):
        c = s[i]
        if in_str:
            cur.append(c)
            if c == "\\" and i + 1 < len(s):
                cur.append(s[i + 1])
                i += 2
                continue
            if c == '"':
                in_str = False
            i += 1
            continue
        if c == '"':
            in_str = True
            cur.append(c)
            i += 1
            continue
        if c == "(":
            depth += 1
            cur.append(c)
            i += 1
            continue
        if c == ")":
            depth -= 1
            cur.append(c)
            i += 1
            continue
        if c.isspace() and depth == 0:
            if cur:
                tokens.append("".join(cur))
                cur = []
            i += 1
            continue
        cur.append(c)
        i += 1
    if cur:
        tokens.append("".join(cur))
    return tokens


def _strip_lisp_pkg(name: str) -> str:
    if "::" in name:
        return name.split("::", 1)[1]
    if name.startswith(":"):
        return name[1:]
    return name


def parse_lisp_lambda_list(s: str | None) -> dict:
    """Count required / optional / key / rest sections of a Lisp lambda list."""
    if s is None:
        return {"required": 0, "optional": 0, "keys": [], "rest": False, "raw": "", "fallback": True}
    s = s.strip()
    if not (s.startswith("(") and s.endswith(")")):
        return {"required": 0, "optional": 0, "keys": [], "rest": False, "raw": s, "fallback": True}
    body = s[1:-1].strip()
    try:
        tokens = _split_top_level_lisp(body)
    except Exception:
        return {"required": 0, "optional": 0, "keys": [], "rest": False, "raw": s, "fallback": True}
    required = 0
    optional = 0
    keys: list[str] = []
    rest = False
    section = "required"
    for tok in tokens:
        low = tok.lower()
        if low == "&optional":
            section = "optional"; continue
        if low == "&rest" or low == "&body":
            section = "rest"; rest = True; continue
        if low == "&key":
            section = "key"; continue
        if low == "&aux":
            section = "aux"; continue
        if low == "&allow-other-keys":
            continue
        if section == "required":
            required += 1
        elif section == "optional":
            optional += 1
        elif section == "key":
            if tok.startswith("("):
                inner = tok[1:-1].strip()
                inner_tokens = _split_top_level_lisp(inner)
                if inner_tokens:
                    keys.append(_strip_lisp_pkg(inner_tokens[0]))
                else:
                    keys.append("?")
            else:
                keys.append(_strip_lisp_pkg(tok))
        # rest / aux: consume silently
    return {"required": required, "optional": optional, "keys": keys, "rest": rest, "raw": s, "fallback": False}


PUB_FN_NAME = re.compile(r"\bpub\s+(?:async\s+)?fn\s+([A-Za-z_][A-Za-z_0-9]*)")

# Matches the canonical ctx-injection first argument per CONVENTIONS §4.8 —
# `ctx: &KaniranContext` (with optional whitespace, lifetime, or `mut`).
# `Arc<KaniranContext>` is also accepted for the (rare) ports that need an
# owned handle rather than a borrow.
CTX_FIRST_ARG = re.compile(
    r"^\s*ctx\s*:\s*"
    r"(?:&(?:'[a-z_]+\s+)?(?:mut\s+)?KaniranContext"
    r"|Arc\s*<\s*KaniranContext\s*>)\s*$"
)


def _split_top_level_rust_args(arglist: str) -> list[str]:
    """Split a Rust args list on top-level commas, respecting <...>, (...), [...]
    nesting. Replaces `->` first so closure-return arrows don't confuse the
    depth counter (mirrors `_rust_count_args`)."""
    s = arglist.replace("->", "@@")
    s = s.strip().rstrip(",").strip()
    if not s:
        return []
    parts: list[str] = []
    cur: list[str] = []
    depth = 0
    for c in s:
        if c in "(<[":
            depth += 1
        elif c in ")>]":
            depth -= 1
        if c == "," and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(c)
    if cur:
        parts.append("".join(cur))
    # Restore the `->` we masked.
    return [p.replace("@@", "->") for p in parts]


def _rust_count_args(arglist: str) -> int:
    """Count top-level comma-separated args in a Rust signature body."""
    return len(_split_top_level_rust_args(arglist))


def _first_arg_is_ctx(arglist: str) -> bool:
    """True iff the first positional arg matches the §4.8 ctx-injection
    convention. Uses the same top-level split as arity counting so a
    generic-typed first arg (e.g. `x: HashMap<K, V>`) doesn't trip the
    bare `,` regex."""
    args = _split_top_level_rust_args(arglist)
    if not args:
        return False
    return bool(CTX_FIRST_ARG.match(args[0]))


def _skip_balanced(text: str, i: int, open_c: str, close_c: str) -> int:
    """Walk past a balanced `open_c ... close_c` starting at text[i] == open_c.
    Returns the index just after the matching close. Caller checks bounds."""
    depth = 1
    i += 1
    while i < len(text) and depth > 0:
        if text[i] == open_c:
            depth += 1
        elif text[i] == close_c:
            depth -= 1
        i += 1
    return i


def parse_rust_pub_fns(text: str) -> list[dict]:
    """Find each `pub fn` / `pub async fn` declaration and count its top-level
    arguments. Walks optional generic params (`<...>`, possibly nested) before
    the arglist."""
    out: list[dict] = []
    for m in PUB_FN_NAME.finditer(text):
        name = m.group(1)
        i = m.end()
        while i < len(text) and text[i].isspace():
            i += 1
        if i < len(text) and text[i] == "<":
            i = _skip_balanced(text, i, "<", ">")
            while i < len(text) and text[i].isspace():
                i += 1
        if i >= len(text) or text[i] != "(":
            continue
        i += 1
        start = i
        depth = 1
        end = -1
        while i < len(text):
            c = text[i]
            if c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0:
                    end = i
                    break
            i += 1
        if end == -1:
            continue
        body = text[start:end]
        out.append({
            "name": name,
            "arity": _rust_count_args(body),
            "ctx_injected": _first_arg_is_ctx(body),
        })
    return out


def lambda_list_from_ftype(ftype: str | None) -> str | None:
    """Extract the args portion of a declared ftype as a parseable lambda list.

    `(function (T1 T2 &key (:k T)) (values ...))` → `(T1 T2 &key (:k T))`.
    Returns None when the ftype is missing, malformed, or has nil args
    (`(function nil ...)` → `()`).
    """
    if not ftype:
        return None
    s = ftype.strip()
    if not (s.startswith("(") and s.endswith(")")):
        return None
    body = s[1:-1].strip()
    tokens = _split_top_level_lisp(body)
    if len(tokens) < 2 or tokens[0].lower() != "function":
        return None
    arg_section = tokens[1]
    if arg_section.lower() == "nil":
        return "()"
    if not (arg_section.startswith("(") and arg_section.endswith(")")):
        return None
    return arg_section


def _audit_sweep(syms: dict[str, dict]) -> tuple[int, int, list[tuple[str, str, str]]]:
    """Full audit sweep — returns (fn_gf_checked, macros_skipped, divergences).

    Each divergence is `(fqn, rel_rust_path, message)`. The Rust path is `""`
    when no port file was found. Sorted by FQN for stable file output.
    """
    if not SIGNATURES_JSON.exists():
        sys.exit(
            "missing signatures.json — run "
            "`python3 reverse/scripts/build_graph.py --signatures-only` first"
        )
    sigs = json.loads(SIGNATURES_JSON.read_text(encoding="utf-8"))

    divergences: list[tuple[str, str, str]] = []
    checked = 0
    skipped_macros = 0
    by_file_pubfns: dict[str, list[dict]] = {}

    for fqn, sym in syms.items():
        if sym["status"] != "ported":
            continue
        kind = sym["kind"]
        if kind not in ("fn", "macro", "gf"):
            continue
        rel_path = fqn_to_rust_path(fqn, kind)
        rust_path = KANIRAN_CORE_SRC / rel_path
        if not rust_path.exists():
            divergences.append((fqn, "", f"port file not found: kaniran-core/src/{rel_path}"))
            continue
        if rel_path not in by_file_pubfns:
            text = rust_path.read_text(encoding="utf-8")
            by_file_pubfns[rel_path] = parse_rust_pub_fns(text)
        pub_fns = by_file_pubfns[rel_path]

        if kind == "macro":
            # Per CONVENTIONS §4.8 macros usually port to doc-only files;
            # only verify the file exists.
            skipped_macros += 1
            continue

        sig = sigs.get(fqn)
        if sig is None:
            divergences.append((fqn, rel_path, "no entry in signatures.json"))
            continue
        ll = sig["lambda_list"] or lambda_list_from_ftype(sig["ftype"])
        info = parse_lisp_lambda_list(ll)
        if info["fallback"]:
            divergences.append(
                (fqn, rel_path,
                 f"lambda-parse-fallback: lambda_list={sig['lambda_list']!r}, "
                 f"ftype={sig['ftype']!r}")
            )
            continue
        expected_arity = info["required"] + info["optional"] + len(info["keys"])
        expected_name = _translate_chars(sym["name"].lower())

        match_fn = next((f for f in pub_fns if f["name"] == expected_name), None)
        if match_fn is None:
            names = [f["name"] for f in pub_fns]
            divergences.append((fqn, rel_path, f"no `pub [async] fn {expected_name}` (found: {names})"))
            continue
        # Per CONVENTIONS §4.8 a ctx: &KaniranContext first parameter is the
        # codified port-wide divergence — not interesting per-fn drift. Adjust
        # the comparison so only the *additional* shape changes (dropped
        # keywords, &rest expansion, etc.) surface as entries.
        effective_arity = match_fn["arity"] - (1 if match_fn["ctx_injected"] else 0)
        if effective_arity != expected_arity:
            ctx_note = " (ctx-injected; +1 absorbed)" if match_fn["ctx_injected"] else ""
            divergences.append(
                (fqn, rel_path,
                 f"arity {match_fn['arity']} ≠ Lisp {expected_arity} "
                 f"(req={info['required']}, opt={info['optional']}, keys={info['keys']})"
                 f"{ctx_note}")
            )
        extras = [f["name"] for f in pub_fns if f["name"] != expected_name]
        if extras:
            divergences.append((fqn, rel_path, f"extra `pub fn`(s) in same file: {extras}"))
        checked += 1

    divergences.sort(key=lambda r: (r[0], r[2]))
    return checked, skipped_macros, divergences


def _render_divergences_md(checked: int, skipped_macros: int,
                           divergences: list[tuple[str, str, str]]) -> str:
    """Render the audit result as markdown. Deterministic across runs (no
    timestamps; entries sorted by FQN); designed to diff cleanly when committed."""
    lines: list[str] = []
    lines.append("# Audit divergences")
    lines.append("")
    lines.append("Auto-generated by `python3 reverse/scripts/query.py audit-signatures`.")
    lines.append("Commit alongside port files; see CONVENTIONS §7.")
    lines.append("")
    lines.append("Each entry is a Rust port whose `pub fn` surface differs from the")
    lines.append("captured Lisp lambda list. New entries should be either:")
    lines.append("")
    lines.append("- intentional, citing a CONVENTIONS section (e.g. §4.4 enum collapse,")
    lines.append("  §4.6 dropped `:fresh`); or")
    lines.append("- a bug in the port to fix.")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Checked: {checked} fn/gf")
    lines.append(f"- Macros (file-existence only): {skipped_macros}")
    lines.append(f"- Divergences: {len(divergences)}")
    lines.append("")
    if not divergences:
        lines.append("No divergences detected.")
        lines.append("")
        return "\n".join(lines)
    lines.append("## Divergences")
    lines.append("")
    for fqn, rel_path, msg in divergences:
        lines.append(f"### `{fqn}`")
        lines.append("")
        if rel_path:
            lines.append(f"- file: `kaniran-core/src/{rel_path}`")
        lines.append(f"- drift: {msg}")
        lines.append("")
    return "\n".join(lines)


def cmd_audit_signatures(args, syms, callees, callers):
    checked, skipped_macros, divergences = _audit_sweep(syms)

    if not args.no_write:
        DIVERGENCES_MD.write_text(_render_divergences_md(checked, skipped_macros, divergences),
                                  encoding="utf-8")

    only = set((args.only or "").split(",")) if args.only else None
    if only:
        shown = [d for d in divergences if any(d[0].startswith(p + ":") for p in only)]
    else:
        shown = divergences

    print(f"audit-signatures: {checked} fn/gf checked, {skipped_macros} macro(s) file-only, "
          f"{len(divergences)} divergence(s) total")
    if not args.no_write:
        rel = os.path.relpath(DIVERGENCES_MD, HERE.parent.parent)
        print(f"  wrote {rel}")
    if only:
        print(f"  filtered to {sorted(only)}: {len(shown)} shown")
    if not shown:
        print("  no divergences" + (" in filtered scope" if only else ""))
        return
    for fqn, _rel, msg in shown:
        print(f"    {fqn}")
        print(f"      {msg}")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)

    sub.add_parser("leaves").set_defaults(fn=cmd_leaves)
    sub.add_parser("next").set_defaults(fn=cmd_next)

    p = sub.add_parser("dependents")
    p.add_argument("fqn")
    p.add_argument("--deep", action="store_true", help="transitive closure")
    p.set_defaults(fn=cmd_dependents)

    p = sub.add_parser("deps")
    p.add_argument("fqn")
    p.add_argument("--deep", action="store_true", help="transitive closure")
    p.set_defaults(fn=cmd_deps)

    sub.add_parser("layers").set_defaults(fn=cmd_layers)

    p = sub.add_parser("mark")
    p.add_argument("fqns", nargs="+")
    p.add_argument("--status", default="ported", help="status to set (default: ported)")
    p.add_argument("--reason", default=None,
                   help="free-form note (recommended for skip/wip). Omit to leave any existing reason untouched.")
    p.set_defaults(fn=cmd_mark)

    p = sub.add_parser(
        "extracted",
        help="tag fns whose parquet fixtures have been captured (--corpus tatoeba/init-suffixes/...).",
    )
    p.add_argument("fqns", nargs="+")
    p.add_argument("--corpus", default=None,
                   help="extraction driver name (e.g. 'tatoeba', 'init-suffixes', 'conj-probe').")
    p.add_argument("--reset", action="store_true", help="clear the extracted column instead of setting it")
    p.set_defaults(fn=cmd_extracted)

    p = sub.add_parser(
        "audited",
        help="tag fns whose tatoeba-pipeline parquet fixtures replayed cleanly (or with N failures). "
             "Reserved for the tatoeba → parquet → audit_fixtures/audit_dict_fixtures pipeline; "
             "non-tatoeba captures stay extracted-only.",
    )
    p.add_argument("fqns", nargs="+")
    p.add_argument("--pass", dest="pass_count", type=int, default=None,
                   help="number of rows that passed replay")
    p.add_argument("--total", type=int, default=None,
                   help="total rows replayed")
    p.add_argument("--reset", action="store_true", help="clear the audited column instead of setting it")
    p.set_defaults(fn=cmd_audited)

    sub.add_parser("stats").set_defaults(fn=cmd_stats)

    p = sub.add_parser("audit-signatures",
                       help="cross-check ported fn/gf signatures against signatures.json")
    p.add_argument("--only", default=None,
                   help="comma-separated package prefixes to scope STDOUT output "
                        "(e.g. 'ichiran/numbers'). The full sweep always runs and "
                        "writes divergences.md regardless.")
    p.add_argument("--no-write", action="store_true",
                   help="don't update reverse/scripts/divergences.md (default: always write)")
    p.set_defaults(fn=cmd_audit_signatures)

    p = sub.add_parser("plan", help="topological port order with SCCs grouped")
    p.add_argument("--out", help="write to file instead of stdout (markdown)")
    p.add_argument("--skip-packages",
                   default="ichiran/test,ichiran/maintenance",
                   help="comma-separated packages to exclude (default: test+maintenance)")
    p.add_argument("--include-status", default=None,
                   help="comma-separated statuses to include (default: all)")
    p.set_defaults(fn=cmd_plan)

    args = ap.parse_args()
    syms, callees, callers = load()
    args.fn(args, syms, callees, callers)
    return 0


if __name__ == "__main__":
    sys.exit(main())

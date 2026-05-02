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
import sys
from collections import defaultdict
from pathlib import Path
from typing import Iterator

HERE = Path(__file__).resolve().parent
SYMBOLS_CSV = HERE / "symbols.csv"
EDGES_CSV = HERE / "edges.csv"


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
    return f"{sym['fqn']:<55} {sym['kind']:<5} {sym['file']}:{sym['line']}  [{sym['status']}{suffix}]"


def _badge(sym: dict) -> str:
    """Plan-line badge for non-pending symbols. Shows the reason when present
    so a reader of PORT_PLAN.md can see why something is off the books."""
    reason = sym.get("reason", "")
    if reason:
        return f"  *[{sym['status']} — {reason}]*"
    return f"  *[{sym['status']}]*"


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


def cmd_mark(args, syms, callees, callers):
    targets = [resolve_fqn(t, syms) for t in args.fqns]
    for t in targets:
        syms[t]["status"] = args.status
        if args.reason is not None:
            syms[t]["reason"] = args.reason
    fields = ["fqn", "name", "package", "file", "line", "kind", "status", "reason"]
    with SYMBOLS_CSV.open("w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=fields)
        w.writeheader()
        for r in sorted(syms.values(), key=lambda r: r["fqn"]):
            # Preserve existing reason for rows not touched; ensure column
            # exists for old CSVs that pre-date the reason field.
            r.setdefault("reason", "")
            w.writerow(r)
    detail = f" with reason {args.reason!r}" if args.reason is not None else ""
    print(f"marked {len(targets)} symbol(s) as {args.status}{detail}", file=sys.stderr)


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

    for i, comp in enumerate(sccs):
        members = sorted(comp)
        if len(members) == 1:
            s = syms[members[0]]
            badge = "" if s["status"] == "pending" else _badge(s)
            out_lines.append(
                f"{i+1:>4}. `{s['fqn']}`  — {s['kind']}, "
                f"{s['file']}:{s['line']}{badge}"
            )
        else:
            out_lines.append(
                f"{i+1:>4}. **CYCLE ({len(members)} symbols — port together)**"
            )
            for fqn in members:
                s = syms[fqn]
                badge = "" if s["status"] == "pending" else _badge(s)
                out_lines.append(
                    f"        - `{s['fqn']}`  — {s['kind']}, "
                    f"{s['file']}:{s['line']}{badge}"
                )

    text = "\n".join(out_lines) + "\n"
    if args.out:
        Path(args.out).write_text(text, encoding="utf-8")
        print(f"wrote {args.out} ({len(sccs)} waves, {len(nodes)} symbols)", file=sys.stderr)
    else:
        sys.stdout.write(text)


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

    sub.add_parser("stats").set_defaults(fn=cmd_stats)

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

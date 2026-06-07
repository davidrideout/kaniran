#!/usr/bin/env python3
"""Pre-clean pass, phase 1: make module-level OnceLock statics function-local.

Each per-symbol file currently declares `static NAME: T = OnceLock::new();` at
module level, used by exactly one accessor fn. When files merge into one module,
the generic names (CACHE, SCANNER, KANJI_CHAR_SCANNER) collide. Moving each
static inside its sole accessor fn (function-local) de-clashes them with zero
behavior change and matches the c113a15 exemplar style.

Deterministic text transform — never rewrites bodies. Verified precondition:
exactly one module-level OnceLock static per file, used in one fn (see the
clash scan). cargo is the net.
"""
import re
import pathlib

SRC = pathlib.Path("kaniran-core/src")
STATIC = re.compile(r'^static\s+([A-Z][A-Z0-9_]*)\s*:\s*(.+?)\s*=\s*OnceLock::new\(\);\s*$')
FN = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+[A-Za-z_]')

changed = []
for f in sorted(SRC.rglob("*.rs")):
    if f.name in ("mod.rs", "tests.rs"):
        continue
    lines = f.read_text(encoding="utf-8").splitlines()
    sidx = [i for i, l in enumerate(lines) if STATIC.match(l)]
    if not sidx:
        continue
    assert len(sidx) == 1, f"{f}: expected 1 module-level OnceLock static, found {len(sidx)}"
    s = sidx[0]
    m = STATIC.match(lines[s])
    name, ty = m.group(1), m.group(2)
    decl = f"    static {name}: {ty} = OnceLock::new();"
    # accessor = first fn declared after the static
    fdecl = next(i for i in range(s + 1, len(lines)) if FN.match(lines[i]))
    # body opens at the first line from the fn decl that contains '{'
    bopen = next(i for i in range(fdecl, len(lines)) if "{" in lines[i])

    new = lines[:]
    new.insert(bopen + 1, decl)   # insert as first line of fn body (bopen > s, so s stays valid)
    del new[s]                    # remove the module-level static
    # collapse a double blank left where the static was
    if 0 < s < len(new) and new[s].strip() == "" and new[s - 1].strip() == "":
        del new[s]
    f.write_text("\n".join(new) + "\n", encoding="utf-8")
    changed.append((str(f.relative_to(SRC)), name))

for path, name in changed:
    print(f"  {name:18} -> fn-local in {path}")
print(f"\n{len(changed)} files transformed")

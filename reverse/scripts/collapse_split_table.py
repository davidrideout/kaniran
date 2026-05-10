#!/usr/bin/env python3
"""Read every kaniran-core/src/dict/split_*.rs file, extract its
`SplitDef { ... }` literal, and emit a single `SPLIT_TABLE` array that
the dispatcher can iterate over directly. Helper `fn` declarations
needed by `Len::Compute(...)` are pulled along too.

Run from repo root:
  python3 reverse/scripts/collapse_split_table.py
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

DICT_DIR = Path(__file__).resolve().parent.parent.parent / "kaniran-core/src/dict"

# Capture: standalone `fn <name>(...) -> Option<usize> { ... }` defined
# above the static. There are exactly 4 such fns across the 3 conditional
# split files.
HELPER_RE = re.compile(
    r"^(fn \w+\(txt: &str, _len_: usize\) -> Option<usize> \{\n.*?\n\})",
    re.MULTILINE | re.DOTALL,
)
# Capture: the SplitDef literal block, minus the leading `pub static DEF: SplitDef = `
# and trailing `;\n`. Result is `SplitDef { ... }` ready to drop into an array.
DEF_RE = re.compile(
    r"^pub static DEF: SplitDef = (SplitDef \{.*?\n\});\n",
    re.MULTILINE | re.DOTALL,
)


def extract(path: Path) -> tuple[list[str], str]:
    """Return (helper_fn_decls, splitdef_literal) from a converted file."""
    text = path.read_text(encoding="utf-8")
    helpers = HELPER_RE.findall(text)
    m = DEF_RE.search(text)
    if not m:
        raise ValueError(f"{path.name}: no SplitDef literal")
    return helpers, m.group(1)


def main() -> int:
    files = sorted(DICT_DIR.glob("split_*.rs"))
    helpers: list[tuple[str, str]] = []   # (origin_filename, decl) — origin used to
                                          # rename helpers per-file (avoid collisions)
    defs: list[tuple[Path, str]] = []
    for path in files:
        try:
            file_helpers, def_lit = extract(path)
            for h in file_helpers:
                helpers.append((path.stem, h))
            defs.append((path, def_lit))
        except Exception as e:
            print(f"  skip {path.name}: {e}", file=sys.stderr)
            return 2

    # Rename each helper to `<origin>__<original_name>` to avoid collisions.
    helper_decls: list[str] = []
    helper_renames: dict[tuple[str, str], str] = {}    # (origin, old) -> new
    for origin, decl in helpers:
        m = re.match(r"fn (\w+)\(", decl)
        if not m:
            raise RuntimeError(f"can't parse helper name: {decl}")
        old_name = m.group(1)
        new_name = f"{origin}__{old_name}"
        helper_renames[(origin, old_name)] = new_name
        decl_renamed = decl.replace(f"fn {old_name}(", f"fn {new_name}(", 1)
        helper_decls.append(decl_renamed)

    # Now rename Len::Compute references inside each SplitDef to point
    # at the renamed helper.
    out_defs: list[str] = []
    for path, def_lit in defs:
        # The compute references look like `Len::Compute(<name>)` — rewrite
        # using the per-file rename map.
        renamed = def_lit
        for (origin, old_name), new_name in helper_renames.items():
            if origin != path.stem:
                continue
            renamed = renamed.replace(f"Len::Compute({old_name})", f"Len::Compute({new_name})")
        out_defs.append("    " + renamed.replace("\n", "\n    ") + ",")

    sorted_defs = sorted(
        out_defs,
        key=lambda d: int(re.search(r"seq: (-?\d+)", d).group(1)),
    )

    body = (
        "//! Port of `ichiran/dict:*split-map*` (`dict-split.lisp:5`).\n"
        "//!\n"
        "//! Hashtable mapping JMdict seq → split function, registered upstream\n"
        "//! by `defsplit` (`dict-split.lisp:7`) which is in turn invoked by\n"
        "//! every `def-simple-split` / `def-de-split` / `def-toori-split` /\n"
        "//! `def-do-split` / `def-shi-split` form. The Rust transliteration\n"
        "//! collapses the runtime hashtable into a static [`SPLIT_TABLE`] of\n"
        "//! data rows. Each row is interpreted by\n"
        "//! [`super::kani_split_engine::run_split`]. Returning `None` from\n"
        "//! [`split_map_dispatch`] for unregistered seqs preserves the upstream\n"
        "//! `(gethash seq *split-map*)` semantics that\n"
        "//! [`super::get_split_star_::get_split_star_`] depends on.\n"
        "//!\n"
        "//! Diverges from CONVENTIONS §1 (one Lisp symbol per Rust file): the\n"
        "//! 174 `split-*` callsites would otherwise need 174 separate\n"
        "//! `dict/split_*.rs` files containing nothing but data rows. Putting\n"
        "//! them here keeps the data and dispatcher together and removes the\n"
        "//! file-per-callsite scaffolding that previously templated future\n"
        "//! `def-simple-split` ports into per-file copies of the same\n"
        "//! interpreter loop. `audit-signatures` will report each `split-*`\n"
        "//! FQN as `port file not found` — those entries are this convention.\n"
        "\n"
        "use crate::conn::kani_context::KaniranContext;\n"
        "use crate::dict::kani_split_engine::{\n"
        "    run_split, Finder, Len, Modify, PartSeq, Pred, ScorePush, SplitDef, Step, WordPart,\n"
        "};\n"
        "use crate::dict::kani_split_part::SplitPart;\n"
        "use crate::dict::kani_word::KaniSimpleTextDispatchEnum;\n"
        "use crate::dict::word_type::WordType;\n"
        "\n"
        + "\n\n".join(helper_decls)
        + "\n\npub static SPLIT_TABLE: &[SplitDef] = &[\n"
        + "\n".join(sorted_defs)
        + "\n];\n"
        "\n"
        "pub async fn split_map_dispatch(\n"
        "    seq: i32,\n"
        "    ctx: &KaniranContext,\n"
        "    reading: &KaniSimpleTextDispatchEnum,\n"
        ") -> Option<Result<(Vec<Option<SplitPart>>, i32), sqlx::Error>> {\n"
        "    let def = SPLIT_TABLE.iter().find(|d| d.seq == seq)?;\n"
        "    Some(run_split(def, ctx, reading).await)\n"
        "}\n"
        "\n"
        "/// Number of registered seqs — pinned so the build fails loudly if\n"
        "/// a future macro form accidentally drops out of the regenerated set.\n"
        "#[cfg(test)]\n"
        "pub(crate) const REGISTERED_COUNT: usize = SPLIT_TABLE.len();\n"
        "\n"
        "#[cfg(test)]\n"
        "mod tests {\n"
        "    use super::*;\n"
        "\n"
        "    #[test]\n"
        "    fn registered_count_matches_upstream_split_map() {\n"
        "        // dict-split.lisp registers 174 entries via def-simple-split /\n"
        "        // def-de-split / def-toori-split / def-do-split /\n"
        "        // def-shi-split outside the *segsplit-map* let-binding.\n"
        "        assert_eq!(REGISTERED_COUNT, 174);\n"
        "    }\n"
        "}\n"
    )

    out_path = DICT_DIR / "_star_split_map_star_.rs"
    out_path.write_text(body, encoding="utf-8")
    print(f"wrote {out_path} ({len(defs)} entries, {len(helper_decls)} helpers)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

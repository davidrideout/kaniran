#!/usr/bin/env python3
"""Convert each kaniran-core/src/dict/split_*.rs from a hand-expanded
50-line `def-simple-split` body into a small data-row shim that
delegates to `crate::dict::kani_split_engine::run_split`.

Strategy: read the existing file, parse the structural shape into a
SplitDef AST, regenerate the shim. Aborts if any pattern doesn't match
the recognized shapes — the caller patches the file by hand and re-runs.

Run from repo root:
  python3 reverse/scripts/convert_split_files.py        # all files
  python3 reverse/scripts/convert_split_files.py FILE   # one file (debug)
"""
from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

DICT_DIR = Path(__file__).resolve().parent.parent.parent / "kaniran-core/src/dict"


@dataclass
class TestStep:
    pred: str         # Rust expression for Pred enum variant (e.g. "Pred::WordType(WordType::Kana)")
    score_mod: Optional[int]
    push: Optional[str]   # "ScorePush::Score" or "ScorePush::PScore" or None


@dataclass
class PushStep:
    push: str         # "ScorePush::Score" or "ScorePush::PScore"


@dataclass
class WordStep:
    pseq_static: Optional[list[int]]  # one of these two is set
    pseq_dynamic: Optional[tuple[str, int]]   # (text, seq)
    length: str       # Rust expression for Len enum variant
    finder: str       # "Finder::Seq" or "Finder::ConjOf"
    modify: str       # Rust expression for Modify enum variant


@dataclass
class SplitFile:
    seq: int
    name: str         # split_1314600 etc.
    score: int
    steps: list       # list of Test/Push/Word
    citation: str     # e.g. "dict-split.lisp:464"


HEADER_RE = re.compile(
    r"//! Port of `ichiran/dict:[^`]+` \(`(dict-split\.lisp:\d+)`\)\."
)
FN_NAME_RE = re.compile(r"pub async fn (split_\w+)\(")
SCORE_RE = re.compile(r"let (?:mut )?score: i32 = (-?\d+);")
TEST_PRED_RE = re.compile(r"^\s*if !\((.*?)\) \{\s*$")
TEST_SCORE_MOD_RE = re.compile(r"^\s*score = (-?\d+);\s*$")
TEST_PUSH_RE = re.compile(r"^\s*parts\.push\(Some\(SplitPart::(Score|PScore)\)\);\s*$")
TEST_RETURN_RE = re.compile(r"^\s*return Ok\(\(parts, score\)\);\s*$")
PUSH_STANDALONE_RE = re.compile(r"^\s*parts\.push\(Some\(SplitPart::(Score|PScore)\)\);\s*$")
PSEQ_STATIC_RE = re.compile(r"let pseq: &\[i32\] = &\[(.*?)\];")
PSEQ_DYN_LOOKUP_RE = re.compile(r'let pseq_lookup = find_word_conj_of\(ctx, "(.*?)", &\[(\d+)i32\]\)\.await\?;')
PART_LENGTH_RE = re.compile(r"let part_length: Option<usize> = (.*?);")
FINDER_RE = re.compile(r"(find_word_seq|find_word_conj_of)\(ctx, &pt_modified, pseq\)\.await\?")
MODIFY_RE = re.compile(r"let pt_modified: String = (.+?);\n")


def parse_pred(expr: str) -> str:
    """Convert a Rust test predicate expression into a Pred enum variant string."""
    expr = expr.strip()
    # strip outer parens (they're redundant from the upstream emitter)
    if expr.startswith("(") and expr.endswith(")"):
        expr = expr[1:-1]
    if expr == "r.word_type() == WordType::Kana":
        return "Pred::WordType(WordType::Kana)"
    if expr == "r.word_type() == WordType::Kanji":
        return "Pred::WordType(WordType::Kanji)"
    m = re.fullmatch(r'txt == "(.*)"', expr)
    if m:
        return f'Pred::TextEquals("{m.group(1)}")'
    m = re.fullmatch(r'txt\.starts_with\("(.*)"\)', expr)
    if m:
        return f'Pred::TextStartsWith("{m.group(1)}")'
    m = re.fullmatch(r"len_ as i32 > (-?\d+)", expr)
    if m:
        return f"Pred::LenGt({m.group(1)})"
    m = re.fullmatch(r"len_ as i32 == (-?\d+)", expr)
    if m:
        return f"Pred::LenEq({m.group(1)})"
    raise ValueError(f"unrecognized test predicate: {expr!r}")


def parse_length(expr: str) -> str:
    """Convert a Rust part-length expression into a Len enum variant string."""
    expr = expr.strip()
    if expr == "None":
        return "Len::Open"
    m = re.fullmatch(r"Some\((\d+)usize\)", expr)
    if m:
        return f"Len::Fixed({m.group(1)})"
    m = re.fullmatch(r"Some\(\(\(len_ as i32 - (\d+)\)\)\.max\(0\) as usize\)", expr)
    if m:
        return f"Len::LenMinus({m.group(1)})"
    m = re.fullmatch(r"txt\.chars\(\)\.position\(\|c\| c == '(.+?)'\)", expr)
    if m:
        return f"Len::CharPos('{m.group(1)}')"
    m = re.fullmatch(r"txt\.chars\(\)\.position\(\|c\| c == '(.+?)'\)\.map\(\|p\| p \+ 1\)", expr)
    if m:
        return f"Len::CharPosPlus1('{m.group(1)}')"
    raise ValueError(f"unrecognized part-length: {expr!r}")


def parse_modify(expr: str) -> str:
    """Convert a Rust pt_modified expression into a Modify enum variant string."""
    expr = expr.strip()
    if expr == "pt.clone()":
        return "Modify::None"
    if "unrendaku::unrendaku" in expr:
        return "Modify::Unrendaku"
    m = re.search(r'optprefix::optprefix\("(.+?)"\)', expr)
    if m:
        return f'Modify::OptPrefix("{m.group(1)}")'
    raise ValueError(f"unrecognized modify: {expr!r}")


def parse_word_block(block: str) -> WordStep:
    """Parse one `{ ... }` word-part block into a WordStep."""
    pseq_static = None
    pseq_dynamic = None
    m = PSEQ_STATIC_RE.search(block)
    if m:
        items = [int(x.strip().rstrip("i32")) for x in m.group(1).split(",")]
        pseq_static = items
    else:
        m = PSEQ_DYN_LOOKUP_RE.search(block)
        if m:
            pseq_dynamic = (m.group(1), int(m.group(2)))
        else:
            raise ValueError(f"can't parse pseq in block:\n{block}")
    m = PART_LENGTH_RE.search(block)
    if not m:
        raise ValueError(f"missing part_length in block:\n{block}")
    length = parse_length(m.group(1))
    m = FINDER_RE.search(block)
    if not m:
        raise ValueError(f"missing finder in block:\n{block}")
    finder = "Finder::Seq" if m.group(1) == "find_word_seq" else "Finder::ConjOf"
    m = MODIFY_RE.search(block)
    if not m:
        raise ValueError(f"missing modify in block:\n{block}")
    modify = parse_modify(m.group(1))
    return WordStep(
        pseq_static=pseq_static,
        pseq_dynamic=pseq_dynamic,
        length=length,
        finder=finder,
        modify=modify,
    )


def split_into_blocks(body: str) -> list[str]:
    """Split the body into top-level `{ ... }` blocks (the word-part ones).

    Naive brace counter starting from each `{` after a blank line."""
    blocks = []
    i = 0
    while i < len(body):
        # find a line that's just `    {` (4 spaces + brace)
        line_start = body.find("\n    {\n", i)
        if line_start == -1:
            break
        # walk braces from here
        start = line_start + 1   # skip leading newline
        depth = 0
        j = start
        while j < len(body):
            if body[j] == "{":
                depth += 1
            elif body[j] == "}":
                depth -= 1
                if depth == 0:
                    blocks.append(body[start:j+1])
                    i = j + 1
                    break
            j += 1
        else:
            break
    return blocks


def parse_split_file(path: Path) -> SplitFile:
    text = path.read_text(encoding="utf-8")
    m = HEADER_RE.search(text)
    if not m:
        raise ValueError(f"{path.name}: missing header citation")
    citation = m.group(1)
    m = FN_NAME_RE.search(text)
    if not m:
        raise ValueError(f"{path.name}: missing pub fn name")
    name = m.group(1)
    # extract seq from name suffix when name is split_<digits>; otherwise
    # the seq lives in `pseq.contains(&<seq>i32)` recursion-guards.
    seq_suffix = re.fullmatch(r"split_(\d+)", name)
    if seq_suffix:
        seq = int(seq_suffix.group(1))
    else:
        m = re.search(r"pseq\.contains\(&(\d+)i32\)", text)
        if not m:
            raise ValueError(f"{path.name}: can't extract seq")
        seq = int(m.group(1))
    m = SCORE_RE.search(text)
    if not m:
        raise ValueError(f"{path.name}: missing score")
    score = int(m.group(1))

    # walk the body line-by-line, recognizing tests, standalone pushes,
    # and word-part blocks.
    lines = text.splitlines()
    # find the function body start/end
    fn_start = None
    for idx, ln in enumerate(lines):
        if FN_NAME_RE.search(ln):
            fn_start = idx
            break
    if fn_start is None:
        raise ValueError(f"{path.name}: fn body not found")
    # body opens at the line ending `{`; closes at the matching `}`
    brace_open = None
    for idx in range(fn_start, len(lines)):
        if lines[idx].rstrip().endswith("{") and ") -> Result" in "\n".join(lines[fn_start:idx+1]):
            brace_open = idx
            break
    if brace_open is None:
        raise ValueError(f"{path.name}: fn open-brace not found")
    body_lines = lines[brace_open + 1:]
    # find matching close — depth count
    depth = 1
    body_end = None
    for idx, ln in enumerate(body_lines):
        for ch in ln:
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    body_end = idx
                    break
        if body_end is not None:
            break
    if body_end is None:
        raise ValueError(f"{path.name}: fn close-brace not found")
    body_lines = body_lines[:body_end]
    body = "\n".join(body_lines)

    # parse step-by-step using a simple state machine
    steps = []
    i = 0
    while i < len(body_lines):
        line = body_lines[i]
        # standalone pushes (not preceded by `if !(...)`)
        if PUSH_STANDALONE_RE.match(line):
            # only count as standalone if not inside a test block (we
            # consume those via the test branch first)
            steps.append(PushStep(push=f"ScorePush::{PUSH_STANDALONE_RE.match(line).group(1)}"))
            i += 1
            continue
        m = TEST_PRED_RE.match(line)
        if m:
            pred = parse_pred(m.group(1))
            score_mod = None
            push = None
            j = i + 1
            while j < len(body_lines):
                inner = body_lines[j]
                if TEST_RETURN_RE.match(inner):
                    j += 1
                    break
                m2 = TEST_SCORE_MOD_RE.match(inner)
                if m2:
                    score_mod = int(m2.group(1))
                    j += 1
                    continue
                m2 = TEST_PUSH_RE.match(inner)
                if m2:
                    push = f"ScorePush::{m2.group(1)}"
                    j += 1
                    continue
                if inner.strip() == "}":
                    j += 1
                    break
                j += 1
            steps.append(TestStep(pred=pred, score_mod=score_mod, push=push))
            i = j
            continue
        if line.strip() == "{":
            # find matching close
            depth = 1
            block_lines = [line]
            j = i + 1
            while j < len(body_lines):
                inner = body_lines[j]
                block_lines.append(inner)
                for ch in inner:
                    if ch == "{":
                        depth += 1
                    elif ch == "}":
                        depth -= 1
                        if depth == 0:
                            break
                if depth == 0:
                    break
                j += 1
            block = "\n".join(block_lines)
            steps.append(parse_word_block(block))
            i = j + 1
            continue
        i += 1

    return SplitFile(seq=seq, name=name, score=score, steps=steps, citation=citation)


def render_step(step) -> str:
    if isinstance(step, TestStep):
        sm = "None" if step.score_mod is None else f"Some({step.score_mod})"
        push = "None" if step.push is None else f"Some({step.push})"
        return (
            f"        Step::Test {{\n"
            f"            pred: {step.pred},\n"
            f"            score_mod: {sm},\n"
            f"            push: {push},\n"
            f"        }},"
        )
    if isinstance(step, PushStep):
        return f"        Step::Push({step.push}),"
    if isinstance(step, WordStep):
        if step.pseq_static is not None:
            seqs = ", ".join(f"{s}i32" for s in step.pseq_static)
            seq = f"PartSeq::Static(&[{seqs}])"
        else:
            text, sval = step.pseq_dynamic
            seq = f'PartSeq::Dynamic {{ text: "{text}", seq: {sval} }}'
        return (
            f"        Step::Word(WordPart {{\n"
            f"            seq: {seq},\n"
            f"            length: {step.length},\n"
            f"            finder: {step.finder},\n"
            f"            modify: {step.modify},\n"
            f"        }}),"
        )
    raise ValueError(f"unknown step: {step}")


def render_file(spec: SplitFile, original_text: str) -> str:
    fqn_dashed = spec.name.replace("split_", "split-").replace("_", "-")
    # but the FQN often differs (split-de-1004800 etc.) — pull from header
    m = re.search(r"`ichiran/dict:([^`]+)`", original_text)
    fqn = m.group(1) if m else fqn_dashed

    # `WordType` only imported when a Pred::WordType is in use
    has_word_type = any(isinstance(s, TestStep) and "WordType" in s.pred for s in spec.steps)
    use_lines = [
        "use crate::conn::kani_context::KaniranContext;",
        "use crate::dict::kani_split_engine::{",
        "    run_split, Finder, Len, Modify, PartSeq, ScorePush, SplitDef, Step, WordPart,",
        "};",
    ]
    if has_word_type:
        use_lines.append("use crate::dict::word_type::WordType;")
    has_pred = any(isinstance(s, TestStep) for s in spec.steps)
    needs_pred = has_pred  # Pred is in the engine module
    if has_pred:
        # adjust the import line to include Pred
        use_lines[1] = "use crate::dict::kani_split_engine::{"
        use_lines[2] = "    run_split, Finder, Len, Modify, PartSeq, Pred, ScorePush, SplitDef, Step, WordPart,"
    # also remove unused imports (ScorePush only when Push or Test.push)
    has_score_push = any(
        isinstance(s, PushStep)
        or (isinstance(s, TestStep) and s.push is not None)
        for s in spec.steps
    )
    if not has_score_push:
        use_lines[2] = use_lines[2].replace("ScorePush, ", "")
    # WordPart only when we have a Word step
    has_word = any(isinstance(s, WordStep) for s in spec.steps)
    if not has_word:
        use_lines[2] = use_lines[2].replace(", WordPart", "").replace("WordPart, ", "")
        use_lines[2] = use_lines[2].replace(", Finder", "").replace("Finder, ", "")
        use_lines[2] = use_lines[2].replace(", Len", "").replace("Len, ", "")
        use_lines[2] = use_lines[2].replace(", Modify", "").replace("Modify, ", "")
        use_lines[2] = use_lines[2].replace(", PartSeq", "").replace("PartSeq, ", "")

    use_lines.append("use crate::dict::kani_split_part::SplitPart;")
    use_lines.append("use crate::dict::kani_word::KaniSimpleTextDispatchEnum;")

    steps_rendered = "\n".join(render_step(s) for s in spec.steps)

    return (
        f"//! Port of `ichiran/dict:{fqn}` (`{spec.citation}`).\n"
        f"//!\n"
        f"//! Data row interpreted by [`crate::dict::kani_split_engine::run_split`].\n"
        f"//! Registered for seq `{spec.seq}` in\n"
        f"//! [`crate::dict::_star_split_map_star_::SPLIT_TABLE`].\n"
        f"\n"
        + "\n".join(use_lines)
        + "\n\n"
        f"pub static DEF: SplitDef = SplitDef {{\n"
        f"    seq: {spec.seq},\n"
        f"    score: {spec.score},\n"
        f"    steps: &[\n"
        f"{steps_rendered}\n"
        f"    ],\n"
        f"}};\n"
        f"\n"
        f"pub async fn {spec.name}(\n"
        f"    ctx: &KaniranContext,\n"
        f"    reading: &KaniSimpleTextDispatchEnum,\n"
        f") -> Result<(Vec<Option<SplitPart>>, i32), sqlx::Error> {{\n"
        f"    run_split(&DEF, ctx, reading).await\n"
        f"}}\n"
    )


def main():
    if len(sys.argv) > 1:
        files = [Path(p) for p in sys.argv[1:]]
    else:
        files = sorted(DICT_DIR.glob("split_*.rs"))

    converted = 0
    skipped = 0
    failed: list[tuple[Path, str]] = []
    for path in files:
        text = path.read_text(encoding="utf-8")
        # already converted — `pub static DEF: SplitDef` is the marker
        if "pub static DEF: SplitDef" in text:
            skipped += 1
            continue
        try:
            spec = parse_split_file(path)
            new_text = render_file(spec, text)
            path.write_text(new_text, encoding="utf-8")
            converted += 1
        except Exception as e:
            failed.append((path, str(e)))
    print(f"skipped {skipped} (already converted)")

    print(f"converted {converted}/{len(files)}")
    if failed:
        print(f"failed {len(failed)}:")
        for p, e in failed:
            print(f"  {p.name}: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()

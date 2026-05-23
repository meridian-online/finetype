#!/usr/bin/env python3
# Per spec 2026-05-23-ydf-specialist-geography ac-01: derive the cue list used
# by the YDF specialist extractor's circular-precision guard from Sense's
# actual header_hint() function. Re-run this whenever column.rs changes.
"""Audit Sense's header_hint() cue list.

Reads `crates/finetype-model/src/column.rs::header_hint()` and emits a TSV
of (header_cue, match_kind, sense_type, source_file, source_line) at
`eval/gittables/ydf_extract_cue_audit.tsv`.

The TSV header records the sha256 of `column.rs` so the extractor can
detect drift between when the audit was generated and the current source.
"""

from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
COLUMN_RS = REPO / "crates" / "finetype-model" / "src" / "column.rs"
OUT_TSV = REPO / "eval" / "gittables" / "ydf_extract_cue_audit.tsv"

FN_START_RE = re.compile(r"^fn header_hint\(")


def find_function_bounds(lines: list[str]) -> tuple[int, int]:
    start = next(i for i, l in enumerate(lines) if FN_START_RE.match(l))
    depth = 0
    in_fn = False
    for i in range(start, len(lines)):
        for c in lines[i]:
            if c == "{":
                depth += 1
                in_fn = True
            elif c == "}":
                depth -= 1
                if in_fn and depth == 0:
                    return start, i
    raise RuntimeError("header_hint() function body not terminated")


def parse_exact_match_arms(body: str, body_start_line: int) -> list[tuple]:
    """Parse the `match h { ... }` block's `"a" | "b" => { ... Some("type") }` arms."""
    rows = []
    arm_re = re.compile(
        r'((?:"[^"]*"\s*(?:\|\s*"[^"]*"\s*)*))\s*=>\s*\{([^{}]*?)\}',
        re.DOTALL,
    )
    str_re = re.compile(r'"([^"]*)"')
    ret_re = re.compile(r'return\s+Some\("([^"]+)"\)')
    for m in arm_re.finditer(body):
        cues_block = m.group(1)
        action = m.group(2)
        ret = ret_re.search(action)
        if not ret:
            continue
        sense_type = ret.group(1)
        line_no = body[: m.start()].count("\n") + body_start_line
        for cue_m in str_re.finditer(cues_block):
            cue = cue_m.group(1)
            rows.append((cue, "exact", sense_type, line_no))
    return rows


def parse_substring_blocks(lines: list[str], start_line: int, end_line: int) -> list[tuple]:
    """Parse substring-matcher region (post-`match` block).

    Walks line-by-line tracking brace depth; each top-level `if` is one block.
    For each block, captures every `h.contains/ends_with/starts_with` literal
    and pairs them with every `return Some(...)` the block contains.
    """
    rows = []
    # Leading `!` makes the call a negation guard, not a positive cue.
    contains_re = re.compile(r'(!?)h\.contains\("([^"]+)"\)')
    ends_re = re.compile(r'(!?)h\.ends_with\("([^"]+)"\)')
    starts_re = re.compile(r'(!?)h\.starts_with\("([^"]+)"\)')
    ret_re = re.compile(r'return\s+Some\("([^"]+)"\)')

    block_lines: list[str] = []
    block_start = None
    depth = 0
    in_block = False
    seen_open = False
    for i in range(start_line, end_line + 1):
        ln = lines[i]
        stripped = ln.lstrip()
        if not in_block:
            if (
                stripped.startswith("if ")
                and (
                    "h.contains" in ln
                    or "h.ends_with" in ln
                    or "h.starts_with" in ln
                    or "!disable_" in ln
                )
            ):
                in_block = True
                block_lines = [ln]
                block_start = i + 1
                depth = ln.count("{") - ln.count("}")
                seen_open = "{" in ln
        else:
            block_lines.append(ln)
            depth += ln.count("{") - ln.count("}")
            if "{" in ln:
                seen_open = True
            if seen_open and depth <= 0:
                block_text = "\n".join(block_lines)
                cues: set[tuple[str, str]] = set()
                for kind, pat in (("substring", contains_re), ("ends_with", ends_re), ("starts_with", starts_re)):
                    for m in pat.finditer(block_text):
                        if m.group(1) == "!":  # negation guard, not a positive cue
                            continue
                        cues.add((m.group(2).strip(), kind))
                types = list({m.group(1) for m in ret_re.finditer(block_text)})
                for cue, kind in cues:
                    for typ in types:
                        rows.append((cue, kind, typ, block_start))
                in_block = False
                block_lines = []
                depth = 0
                seen_open = False
    return rows


def audit() -> tuple[list[tuple], str]:
    src_bytes = COLUMN_RS.read_bytes()
    sha = hashlib.sha256(src_bytes).hexdigest()
    src = src_bytes.decode("utf-8")
    lines = src.split("\n")
    fn_start, fn_end = find_function_bounds(lines)

    # Exact-match arms live inside the first `match h { ... }` block.
    match_start = next(
        i for i in range(fn_start, fn_end + 1) if "match h" in lines[i]
    )
    # Find the closing `}` of the match block.
    depth = 0
    in_match = False
    match_end = None
    for i in range(match_start, fn_end + 1):
        for c in lines[i]:
            if c == "{":
                depth += 1
                in_match = True
            elif c == "}":
                depth -= 1
                if in_match and depth == 0:
                    match_end = i
                    break
        if match_end:
            break
    if match_end is None:
        raise RuntimeError("match h { ... } block not terminated")

    match_body = "\n".join(lines[match_start : match_end + 1])
    exact_rows = parse_exact_match_arms(match_body, match_start + 1)

    # Substring matcher region: everything between the close of the if-let
    # containing the match block and the end of header_hint.
    substring_rows = parse_substring_blocks(lines, match_end + 1, fn_end)

    rows = exact_rows + substring_rows
    return rows, sha


def main() -> int:
    rows, sha = audit()
    OUT_TSV.parent.mkdir(parents=True, exist_ok=True)
    with OUT_TSV.open("w", encoding="utf-8") as f:
        f.write(f"# source_sha256: {sha}\n")
        f.write(f"# source_file: crates/finetype-model/src/column.rs\n")
        f.write(f"# generator: scripts/audit_sense_header_cues.py\n")
        f.write(f"# row_count: {len(rows)}\n")
        f.write("header_cue\tmatch_kind\tsense_type\tsource_line\n")
        for cue, kind, sense_type, line_no in sorted(rows):
            f.write(f"{cue}\t{kind}\t{sense_type}\t{line_no}\n")
    print(f"wrote {len(rows)} cue rows to {OUT_TSV}", file=sys.stderr)
    print(f"source_sha256: {sha}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())

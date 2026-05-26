#!/usr/bin/env python3
"""ac-01 — Gate YDF per-column predictions against JSON Schema validation.

For each row in a corpus-pass parquet, look up the taxonomy validation
for `ydf_prediction`, evaluate it against the column's sample values,
and emit two new columns:

  ydf_validation_pass_rate (DOUBLE)
    Fraction of non-empty sample values that pass the validation. 1.0
    when YDF has no prediction, when the predicted label has no
    applicable validation, or when there are no sampled values to check.

  ydf_prediction_gated (VARCHAR, nullable)
    NULL when pass_rate < 0.5 (the YDF prediction is structurally
    contradicted by the values), otherwise the original prediction.

Validation kinds:
  1. validation.pattern + validation.enum    when both present, a value
                                             must pass BOTH the regex
                                             AND be in the enum set —
                                             mirrors the taxonomy's
                                             jsonschema-rs semantics
                                             (CompiledValidator)
  2. validation.pattern                      per-value regex (fullmatch)
  3. validation.enum                         set membership
  4. validation_by_locale                    union of per-locale patterns
                                             (value passes if any locale
                                             matches), only when no
                                             top-level pattern/enum
  5. validation.minimum/maximum              per-value parse + range
  6. (no applicable validation)              pass_rate = 1.0; kept

Length-only validations (minLength/maxLength alone) are intentionally
NOT used — they're too weak to be useful (most strings pass any
length check). The high-leverage types we care about (iso6346, mgrs,
plus_code, country_code) all have patterns.

Pattern + enum joint application matters for country_code: the alpha-2
pattern `^[A-Z]{2}$` accepts 676 codes, but the ISO 3166-1 enum
contains only 249. Without the joint check, the gate would accept
YDF's `country_code` label on US-state-shaped values like `UT`, `OK`,
`OR`, `OH`, `IA` — none of which are real country codes.

The gate is symmetric and label-agnostic: it never decides "this
column IS X", only "the YDF prediction X is contradicted by the
values". Refused predictions become NULL; downstream consumers
(cell-delta scripts, mining) treat NULL as "no signal" rather than
treating it as ground truth.

Run:
    uv run --with duckdb,pyarrow,pyyaml python3 \\
        scripts/apply_ydf_validation_gate.py \\
        --columns output/corpus-pass-v22/corpus_pass/columns.parquet \\
        --out output/ydf-validation-gate/v22_gated.parquet

Per spec 2026-05-26-ydf-validation-gate ac-01.
"""
from __future__ import annotations

import argparse
import glob
import sys
from dataclasses import dataclass, field
from pathlib import Path

# Use the PCRE-compatible `regex` library instead of stdlib `re` —
# several taxonomy patterns use `\p{...}` Unicode property syntax and
# `\xNN` partial escapes that Python's stdlib re doesn't understand.
# Silently dropping those would leave finance.currency.amount* and
# identity.person.full_name (among others) ungated, defeating the
# spec on a wide swathe of types.
import regex as re  # type: ignore

REPO = Path(__file__).resolve().parent.parent
LABELS_DIR = REPO / "labels"

DEFAULT_IN = REPO / "output/corpus-pass-v22/corpus_pass/columns.parquet"
DEFAULT_OUT = REPO / "output/ydf-validation-gate/v22_gated.parquet"

PASS_RATE_THRESHOLD = 0.5

@dataclass
class ValidationSpec:
    """A label's compiled validation criteria.

    `kind` records the primary diagnostic kind. A spec may carry both
    a `pattern_re` and an `enum_set` — when both are set, `passes()`
    requires the value to satisfy BOTH, matching the joint semantics
    of the taxonomy's JSON Schema export (see
    `crates/finetype-core/src/validator.rs::CompiledValidator`).
    """
    label: str
    kind: str  # 'pattern' | 'enum' | 'locale' | 'range' | 'none'
    pattern_re: re.Pattern | None = None
    enum_set: frozenset[str] | None = None
    locale_patterns: list[re.Pattern] = field(default_factory=list)
    minimum: float | None = None
    maximum: float | None = None

    def passes(self, value: str) -> bool:
        if self.kind in ("pattern", "enum"):
            # Joint application — when both attached, both must pass.
            if (self.pattern_re is not None
                    and self.pattern_re.fullmatch(value) is None):
                return False
            if (self.enum_set is not None
                    and value not in self.enum_set):
                return False
            return True
        if self.kind == "locale":
            return any(p.fullmatch(value) is not None
                       for p in self.locale_patterns)
        if self.kind == "range":
            try:
                f = float(value)
            except (ValueError, TypeError):
                return False
            if self.minimum is not None and f < self.minimum:
                return False
            if self.maximum is not None and f > self.maximum:
                return False
            return True
        return True


def load_validations() -> dict[str, ValidationSpec]:
    """Parse labels/definitions_*.yaml into label → ValidationSpec."""
    import yaml  # type: ignore

    specs: dict[str, ValidationSpec] = {}
    for path in sorted(glob.glob(str(LABELS_DIR / "definitions_*.yaml"))):
        with open(path) as fh:
            doc = yaml.safe_load(fh) or {}
        for label, body in doc.items():
            if not isinstance(body, dict):
                continue
            specs[label] = _compile_spec(label, body)
    return specs


def _normalise_pattern(pat: str) -> str:
    """Convert PCRE-style `\\x{NNNN}` Unicode escapes to Python-compatible
    `\\uNNNN`. Several taxonomy patterns use the PCRE braced-hex form
    (e.g. `\\x{00A0}` for NBSP); neither stdlib `re` nor `regex`
    accepts it. We rewrite to the standard 4-digit-Unicode form.
    """
    def sub(m):
        hex_str = m.group(1)
        # Python `\u` requires exactly 4 hex digits; pad short ones.
        if len(hex_str) <= 4:
            return f"\\u{hex_str.zfill(4)}"
        # 5-6 digit code points use `\U00NNNNNN` (8 hex digits).
        return f"\\U{hex_str.zfill(8)}"
    return re.sub(r"\\x\{([0-9A-Fa-f]+)\}", sub, pat)


def _compile_spec(label: str, body: dict) -> ValidationSpec:
    v = body.get("validation") or {}
    by_locale = body.get("validation_by_locale") or {}

    pattern = v.get("pattern")
    enum_vals = v.get("enum")
    minimum = v.get("minimum")
    maximum = v.get("maximum")

    pattern_re: re.Pattern | None = None
    enum_set: frozenset[str] | None = None

    if pattern:
        try:
            pattern_re = re.compile(_normalise_pattern(pattern))
        except re.error as exc:
            print(f"warn: {label} pattern uncompilable ({exc}); skipping",
                  file=sys.stderr)
    if enum_vals:
        enum_set = frozenset(str(x) for x in enum_vals)

    if pattern_re is not None or enum_set is not None:
        # `kind` records the most-constraining attached check for
        # diagnostics — enum wins when both are present (it's the
        # tighter check for fixed-domain types like country_code).
        kind = "enum" if enum_set is not None else "pattern"
        return ValidationSpec(
            label=label, kind=kind,
            pattern_re=pattern_re, enum_set=enum_set)

    if by_locale:
        compiled: list[re.Pattern] = []
        for loc_v in by_locale.values():
            if not isinstance(loc_v, dict):
                continue
            p = loc_v.get("pattern")
            if p:
                try:
                    compiled.append(re.compile(_normalise_pattern(p)))
                except re.error:
                    continue
        if compiled:
            return ValidationSpec(
                label=label, kind="locale", locale_patterns=compiled)

    if minimum is not None or maximum is not None:
        return ValidationSpec(
            label=label, kind="range",
            minimum=float(minimum) if minimum is not None else None,
            maximum=float(maximum) if maximum is not None else None)

    return ValidationSpec(label=label, kind="none")


def evaluate(spec: ValidationSpec | None, values: list[str]) -> float:
    """Return pass_rate ∈ [0, 1].

    pass_rate = 1.0 when there's no applicable validation, no spec,
    or no non-empty values (no evidence either way → keep prediction).
    """
    if spec is None or spec.kind == "none":
        return 1.0
    non_empty = [v.strip() for v in values if v is not None and v.strip()]
    if not non_empty:
        return 1.0
    passes = sum(1 for v in non_empty if spec.passes(v))
    return passes / len(non_empty)


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--columns", type=Path, default=DEFAULT_IN)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb/pyarrow missing ({exc}); run with "
              "`uv run --with duckdb,pyarrow,pyyaml python3 "
              "scripts/apply_ydf_validation_gate.py`", file=sys.stderr)
        return 2

    if not args.columns.exists():
        print(f"error: columns parquet missing at {args.columns}",
              file=sys.stderr)
        return 2

    args.out.parent.mkdir(parents=True, exist_ok=True)

    print(f"loading taxonomy from {LABELS_DIR}", file=sys.stderr)
    specs = load_validations()
    kind_counts: dict[str, int] = {}
    for s in specs.values():
        kind_counts[s.kind] = kind_counts.get(s.kind, 0) + 1
    print(f"  {len(specs)} labels: " +
          ", ".join(f"{k}={v}" for k, v in sorted(kind_counts.items())),
          file=sys.stderr)

    print(f"reading {args.columns}", file=sys.stderr)
    con = duckdb.connect()
    table = con.execute(f"""
        SELECT *
          FROM read_parquet('{args.columns.as_posix()}')
    """).fetch_arrow_table()
    n = table.num_rows
    print(f"  {n:,} columns", file=sys.stderr)

    ydf_preds = table["ydf_prediction"].to_pylist()
    samples_col = table["sample_values_truncated"].to_pylist()

    pass_rates: list[float | None] = [None] * n
    gated: list[str | None] = [None] * n
    refused_by_label: dict[str, int] = {}
    kept_by_label: dict[str, int] = {}

    for i in range(n):
        ydf_pred = ydf_preds[i]
        if ydf_pred is None:
            pass_rates[i] = None
            gated[i] = None
            continue
        spec = specs.get(ydf_pred)
        samples = samples_col[i]
        values = samples.split('│') if samples else []
        rate = evaluate(spec, values)
        pass_rates[i] = rate
        if rate < PASS_RATE_THRESHOLD:
            gated[i] = None
            refused_by_label[ydf_pred] = refused_by_label.get(ydf_pred, 0) + 1
        else:
            gated[i] = ydf_pred
            kept_by_label[ydf_pred] = kept_by_label.get(ydf_pred, 0) + 1

    total_refused = sum(refused_by_label.values())
    total_kept = sum(kept_by_label.values())
    print(f"  refused {total_refused:,} / kept {total_kept:,} "
          f"({total_refused / max(total_refused + total_kept, 1) * 100:.2f}% refused)",
          file=sys.stderr)
    print("  top refused YDF labels:", file=sys.stderr)
    for label, k in sorted(refused_by_label.items(), key=lambda x: -x[1])[:10]:
        kept = kept_by_label.get(label, 0)
        rr = k / (k + kept) * 100 if (k + kept) else 0.0
        print(f"    {label}: refused {k:,} / kept {kept:,} ({rr:.1f}% refusal)",
              file=sys.stderr)

    out_table = table.append_column(
        "ydf_validation_pass_rate", pa.array(pass_rates, type=pa.float64()))
    out_table = out_table.append_column(
        "ydf_prediction_gated", pa.array(gated, type=pa.string()))
    pq.write_table(out_table, args.out)
    print(f"wrote {args.out}  ({out_table.num_rows:,} rows)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())

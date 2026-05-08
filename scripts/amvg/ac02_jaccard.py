#!/usr/bin/env python3
"""ac-02: Pairwise value-shape Jaccard matrix (12x12) for amount subtypes.

For each subtype we draw a deterministic sample of SAMPLES_PER_SUBTYPE values
via `cargo run --example amvg_sample -- <key> <N> <SEED>` (seeded RNG), compress
each value into a character-class shape signature, and compute pairwise Jaccard
similarity on the set of distinct signatures per subtype.

Deterministic: re-running the script produces a byte-identical
diagnostics/jaccard_matrix.tsv given the same (SAMPLES_PER_SUBTYPE, SEED).

Spec v1.2 ac-02 assertions (also enforced here as hard asserts):
  (a) 12 rows x 12 cols; labels match TARGET_SUBTYPES
  (b) matrix symmetric (M[i,j] == M[j,i])
  (c) diagonal == 1.0
  (d) all off-diagonal values in [0.0, 1.0]; NOT all equal
  (e) each subtype has >= 3 distinct signatures
"""
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC_DIR = REPO_ROOT / ".orbit/specs/2026-04-24-amount-variant-generators"
DIAG_DIR = SPEC_DIR / "diagnostics"

SAMPLES_PER_SUBTYPE = 100
SEED = 42  # pinned per review-spec v2 MEDIUM (RNG seeding)

TARGET_SUBTYPES = [
    "finance.currency.amount",
    "finance.currency.amount_accounting",
    "finance.currency.amount_apostrophe",
    "finance.currency.amount_code_prefix",
    "finance.currency.amount_comma",
    "finance.currency.amount_comma_suffix",
    "finance.currency.amount_crypto",
    "finance.currency.amount_lakh",
    "finance.currency.amount_multisym",
    "finance.currency.amount_neg_trailing",
    "finance.currency.amount_nodecimal",
    "finance.currency.amount_space",
]


def shape_signature(s: str) -> str:
    """Compress a value to a character-class shape.

    Each character maps to a class:
      digits        -> 'D'
      upper letters -> 'A'
      lower letters -> 'a'
      other chars   -> preserved verbatim (punctuation, currency symbols,
                       whitespace are load-bearing for amount variants)

    Runs are NOT collapsed: preserving run length distinguishes
    `amount_crypto` tickers of different widths (DOGE vs BTC) and
    preserves digit-grouping cardinality (e.g. `D,DDD` vs `DD,DDD`) that
    is semantically meaningful for amount subtypes.
    """
    out = []
    for c in s:
        if c.isdigit():
            out.append("D")
        elif c.isalpha():
            out.append("A" if c.isupper() else "a")
        else:
            out.append(c)
    return "".join(out)


def sample(key: str) -> list[str]:
    cmd = [
        "cargo",
        "run",
        "--example",
        "amvg_sample",
        "--quiet",
        "--",
        key,
        str(SAMPLES_PER_SUBTYPE),
        str(SEED),
    ]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=REPO_ROOT)
    lines = [ln for ln in out.stdout.splitlines() if ln]
    if len(lines) != SAMPLES_PER_SUBTYPE:
        sys.exit(f"expected {SAMPLES_PER_SUBTYPE} values for {key}, got {len(lines)}")
    return lines


def jaccard(a: set, b: set) -> float:
    if not a and not b:
        return 1.0
    return len(a & b) / len(a | b)


def main() -> None:
    print(f"sampling {SAMPLES_PER_SUBTYPE} values/subtype (seed={SEED})...")
    sig_sets: dict[str, set[str]] = {}
    for key in TARGET_SUBTYPES:
        values = sample(key)
        sigs = {shape_signature(v) for v in values}
        sig_sets[key] = sigs
        print(f"  {key}: {len(sigs)} distinct signatures")

    # Assertion (e): each subtype has >= 3 distinct signatures
    for key, sigs in sig_sets.items():
        assert len(sigs) >= 3, f"ac-02 (e) violation: {key} has only {len(sigs)} distinct signatures"

    # Build 12x12 matrix
    n = len(TARGET_SUBTYPES)
    matrix = [[0.0] * n for _ in range(n)]
    for i, ki in enumerate(TARGET_SUBTYPES):
        for j, kj in enumerate(TARGET_SUBTYPES):
            matrix[i][j] = jaccard(sig_sets[ki], sig_sets[kj])

    # Assertions
    for i in range(n):
        # (c) diagonal == 1.0
        assert matrix[i][i] == 1.0, f"ac-02 (c) violation: diag[{i}] = {matrix[i][i]}"
        for j in range(n):
            # (d) range
            assert 0.0 <= matrix[i][j] <= 1.0, f"ac-02 (d) range violation at [{i},{j}] = {matrix[i][j]}"
            # (b) symmetry
            assert matrix[i][j] == matrix[j][i], f"ac-02 (b) symmetry violation at [{i},{j}]"

    # (d) not-all-equal off-diagonal
    off_diag = [matrix[i][j] for i in range(n) for j in range(n) if i != j]
    assert len(set(f"{v:.6f}" for v in off_diag)) > 1, "ac-02 (d) all off-diag values equal"

    # Write TSV
    DIAG_DIR.mkdir(parents=True, exist_ok=True)
    header = "subtype\t" + "\t".join(TARGET_SUBTYPES) + "\n"
    lines = [header]
    for i, ki in enumerate(TARGET_SUBTYPES):
        cells = [f"{matrix[i][j]:.4f}" for j in range(n)]
        lines.append(ki + "\t" + "\t".join(cells) + "\n")
    (DIAG_DIR / "jaccard_matrix.tsv").write_text("".join(lines))

    # Also print summary stats for the log
    mean_off = sum(off_diag) / len(off_diag)
    max_off = max(off_diag)
    min_off = min(off_diag)
    print(f"jaccard mean(off-diag)={mean_off:.4f} max={max_off:.4f} min={min_off:.4f}")
    print("ac-02 artefact: diagnostics/jaccard_matrix.tsv")


if __name__ == "__main__":
    main()

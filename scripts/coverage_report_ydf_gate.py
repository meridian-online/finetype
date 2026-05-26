#!/usr/bin/env python3
"""ac-03 — Per-type coverage report for the YDF validation gate.

Reads the four gated parquets and emits
`output/ydf-validation-gate/coverage_report.md` showing, per YDF
label, the count of predictions made, refused, and refusal rate.
Sorted by absolute refusal count descending so the noisiest YDF
labels surface first.

Acts as a sanity check on the gate. We expect iso6346, mgrs,
plus_code, credit_card_number, phone_e164 to dominate refusals;
text/numeric labels with valid patterns should have near-zero
refusal rates.

Per spec 2026-05-26-ydf-validation-gate ac-03.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
GATE_DIR = REPO / "output/ydf-validation-gate"
DEFAULT_OUT = GATE_DIR / "coverage_report.md"

VERSIONS = ["v19", "v20", "v21", "v22"]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb missing ({exc}).", file=sys.stderr)
        return 2

    con = duckdb.connect()
    args.out.parent.mkdir(parents=True, exist_ok=True)

    lines: list[str] = []
    lines.append("# YDF validation-gate coverage report\n\n")
    lines.append("Per spec `2026-05-26-ydf-validation-gate` ac-03.\n\n")
    lines.append(
        "For each YDF predicted label, this report shows the count of "
        "predictions made vs refused by the gate, across the four "
        "tracked corpus passes (v19 through v22). Refusal rate is "
        "`refused / (refused + kept)`. The gate refuses a prediction "
        "when fewer than 50% of the column's sampled values pass the "
        "label's JSON Schema validation (per ac-01).\n\n"
    )

    for ver in VERSIONS:
        parquet = GATE_DIR / f"{ver}_gated.parquet"
        if not parquet.exists():
            print(f"warn: {parquet} missing — skipping", file=sys.stderr)
            continue
        print(f"reading {parquet}", file=sys.stderr)

        rows = con.execute(f"""
            WITH labelled AS (
              SELECT ydf_prediction AS label,
                     ydf_prediction_gated IS NULL
                       AND ydf_prediction IS NOT NULL AS refused
                FROM read_parquet('{parquet.as_posix()}')
               WHERE ydf_prediction IS NOT NULL
            )
            SELECT label,
                   COUNT(*) FILTER (WHERE NOT refused) AS kept,
                   COUNT(*) FILTER (WHERE     refused) AS refused,
                   COUNT(*) AS total
              FROM labelled
             GROUP BY 1
             ORDER BY refused DESC, total DESC
        """).fetchall()

        total_pred = sum(r[3] for r in rows)
        total_kept = sum(r[1] for r in rows)
        total_refused = sum(r[2] for r in rows)
        lines.append(f"## {ver}\n\n")
        lines.append(
            f"**{total_pred:,} predictions** — {total_kept:,} kept, "
            f"{total_refused:,} refused "
            f"({total_refused / total_pred * 100:.2f}% refusal rate).\n\n"
        )

        # Top 25 by refused count (the noisy labels)
        lines.append("### Top 25 by refusal count\n\n")
        lines.append("| YDF label | kept | refused | total | refusal % |\n")
        lines.append("|---|---:|---:|---:|---:|\n")
        for label, kept, refused, total in rows[:25]:
            rr = refused / total * 100 if total else 0
            bold = "**" if rr >= 50 else ""
            lines.append(
                f"| {bold}{label}{bold} | {kept:,} | "
                f"{bold}{refused:,}{bold} | {total:,} | {rr:.1f}% |\n"
            )
        lines.append("\n")

        # Sanity-check section — labels we KNOW should be near-100% refused
        # based on the v23 ac-01 finding.
        canary_labels = [
            "geography.transportation.iso6346",
            "geography.coordinate.mgrs",
            "geography.coordinate.plus_code",
            "finance.payment.credit_card_number",
            "identity.person.phone_e164",
        ]
        canary_rows = {r[0]: r for r in rows if r[0] in canary_labels}
        if canary_rows:
            lines.append("### Canary types (expect high refusal)\n\n")
            lines.append("| YDF label | kept | refused | total | refusal % |\n")
            lines.append("|---|---:|---:|---:|---:|\n")
            for label in canary_labels:
                if label not in canary_rows:
                    continue
                _, kept, refused, total = canary_rows[label]
                rr = refused / total * 100 if total else 0
                lines.append(
                    f"| {label} | {kept:,} | {refused:,} | {total:,} | {rr:.1f}% |\n"
                )
            lines.append("\n")

    args.out.write_text("".join(lines))
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())

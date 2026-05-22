#!/usr/bin/env python3
"""ac-10 — synthesise eval/gittables/corpus_pass/report.md.

Reads:
  - eval/gittables/corpus_pass/corroborated_gaps.parquet  (Part 1)
  - eval/gittables/corpus_pass/dbpedia_annotations.parquet (Part 1 enrichment + Part 2)
  - eval/gittables/corpus_pass/files.parquet              (Part 2 criterion-a filter)
  - eval/gittables/corpus_pass/columns.parquet            (Part 2 host-column filter)
  - eval/gittables/dbpedia_finetype_mapping.tsv           (Part 2 no_finetype_equivalent)
  - eval/gittables/corpus_paths.sha256                    (frontmatter)
  - models/default/                                       (frontmatter: model_sha)
  - eval/gittables/models/ydf.bin                         (frontmatter: ydf_sha)

Writes:
  eval/gittables/corpus_pass/report.md

The report is byte-stable across re-runs: no wall-clock timestamp,
all hashes derived from input files. Runtime metadata lives in
progress.md (separate concern).
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
NON_TRIVIAL_FLOOR = 0.80
PART2_MIN_COLS = 10
PART1_TOPN_PER_CELL = 10
PART2_SAMPLE_COLS = 3

# ac-12 outcome (2026-05-23) — cells whose pre-screen pass rate fell
# below the 90% threshold are demoted from Part 1 to
# single_lens_signals.tsv per the spec's failure-consequence procedure.
# Each entry is a (criterion, mechanism) tuple. The audit trail lives
# in spot_check_prescreen.md + spot_check.md + progress.md (2026-05-23).
DEMOTED_CELLS: frozenset[tuple[str, str]] = frozenset({
    ("non_trivial_floor", "format_diversity_path_b"),
    ("reject_rate_ceil",  "code_vs_canonical_path_a"),
    ("reject_rate_ceil",  "format_diversity_path_a"),
    ("reject_rate_ceil",  "validator_widening"),
})

TRIVIAL_TYPES = {
    "representation.text.plain_text",
    "representation.numeric.decimal_number",
}


def file_sha256(path: Path) -> str:
    """SHA256 of a single file's bytes."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def dir_sha256(path: Path) -> str:
    """SHA256 of a directory's contents: hash each file's path+bytes
    in sorted order. Reproducible across re-runs."""
    h = hashlib.sha256()
    for sub in sorted(path.rglob("*")):
        if sub.is_file():
            h.update(str(sub.relative_to(path)).encode("utf-8"))
            h.update(b"\x00")
            with open(sub, "rb") as f:
                for chunk in iter(lambda: f.read(65536), b""):
                    h.update(chunk)
            h.update(b"\x00")
    return h.hexdigest()


def compute_frontmatter_shas(
    model_dir: Path,
    ydf_path: Path,
    dbpedia_mapping_path: Path,
    corpus_index_sha_path: Path,
) -> dict[str, str]:
    """Compute the six reproducibility anchors."""
    # model_sha — Sense model directory hash
    model_sha = dir_sha256(model_dir) if model_dir.exists() else "missing"

    # ydf_sha — YDF model file/directory hash. ydf.bin from YDF may be
    # a directory (extracted bundle); fall back to file.
    if ydf_path.is_dir():
        ydf_sha = dir_sha256(ydf_path)
    elif ydf_path.is_file():
        ydf_sha = file_sha256(ydf_path)
    else:
        ydf_sha = "missing"

    # dbpedia_mapping_sha — the curated TSV
    dbpedia_mapping_sha = (
        file_sha256(dbpedia_mapping_path)
        if dbpedia_mapping_path.exists() else "missing"
    )

    # cascade_version — MADR 0075 + 0081 revision tag. We use a stable
    # composite of the two MADR file SHAs so the field tracks any
    # cascade-rule change without manual bumping.
    madr_paths = [
        REPO / ".orbit/choices/0075-mechanism-cascade.yaml",
        REPO / ".orbit/choices/0081-cascade-extended.yaml",
    ]
    cascade_h = hashlib.sha256()
    cascade_h.update(b"MADR-0075-0081|")
    for p in madr_paths:
        if p.exists():
            cascade_h.update(file_sha256(p).encode("ascii"))
            cascade_h.update(b"|")
    cascade_version = cascade_h.hexdigest()[:16]

    # corpus_index_sha — already pre-computed
    corpus_index_sha = (
        corpus_index_sha_path.read_text().strip().split()[0]
        if corpus_index_sha_path.exists() else "missing"
    )

    # corpus_pass_id — composite per spec hard-constraint 9:
    # SHA256(model_sha || 0x00 || ydf_sha || 0x00 ||
    #         dbpedia_mapping_sha || 0x00 || cascade_version || 0x00 ||
    #         corpus_index_sha)
    h = hashlib.sha256()
    for v in (model_sha, ydf_sha, dbpedia_mapping_sha,
              cascade_version, corpus_index_sha):
        h.update(v.encode("ascii"))
        h.update(b"\x00")
    corpus_pass_id = h.hexdigest()

    return {
        "model_sha": model_sha,
        "ydf_sha": ydf_sha,
        "dbpedia_mapping_sha": dbpedia_mapping_sha,
        "cascade_version": cascade_version,
        "corpus_index_sha": corpus_index_sha,
        "corpus_pass_id": corpus_pass_id,
    }


def fmt_samples(sample_values: list[str], max_n: int = 3, max_len: int = 60) -> str:
    """Compact preview of sample values for markdown table cells."""
    show = sample_values[:max_n]
    out = []
    for s in show:
        s = (s or "").replace("\n", " ").replace("\r", " ").strip()
        if len(s) > max_len:
            s = s[: max_len - 1] + "…"
        # Escape backticks in markdown code spans
        s = s.replace("`", "")
        out.append(f"`{s}`")
    return " ".join(out)


def build_part1(corroborated_path: Path, dbpedia_annotations_path: Path) -> str:
    """Top-10 ranked corroborated gaps per (criterion × mechanism) cell."""
    import duckdb  # type: ignore

    con = duckdb.connect()

    # Load corroborated gaps, sorted within each cell
    gaps = con.execute(f"""
        SELECT *
        FROM read_parquet('{corroborated_path}')
        ORDER BY criterion, mechanism, rank_within_cell
    """).fetch_arrow_table().to_pylist()

    # Build dbpedia annotation lookup: (file_path, column_name) -> class
    # Used to enrich Part 1 sample_evidence with dbpedia context.
    dbp_rows = con.execute(f"""
        SELECT file_path, column_name, dbpedia_semantic_class
        FROM read_parquet('{dbpedia_annotations_path}')
        WHERE dbpedia_semantic_class IS NOT NULL
              AND dbpedia_semantic_class != ''
    """).fetch_arrow_table().to_pylist()
    dbp_lookup: dict[tuple[str, str], str] = {
        (r["file_path"], r["column_name"]): r["dbpedia_semantic_class"]
        for r in dbp_rows
    }

    # Group gaps by cell
    cells: dict[tuple[str, str], list[dict]] = {}
    for g in gaps:
        key = (g["criterion"], g["mechanism"])
        cells.setdefault(key, []).append(g)

    # All possible cells (criterion × mechanism). We list empty cells
    # explicitly per spec.
    closed_mechs = [
        "format_diversity_path_a", "format_diversity_path_b",
        "code_vs_canonical_path_a", "code_vs_canonical_path_b",
        "enum_overfit", "misclassification",
        "validator_widening", "unknown_no_fit", "fallthrough",
    ]
    criteria = ["non_trivial_floor", "reject_rate_ceil"]

    out: list[str] = []
    out.append("## Part 1 — Corroborated gaps (the headline diagnostic)")
    out.append("")
    out.append("Each section below is one `(criterion × mechanism)` cell. "
               "Top-10 ranked gap clusters per cell, where a *gap cluster* "
               "groups columns sharing the same mechanism, taxonomy "
               "prediction, and value shape signature. Each cluster has "
               "been independently flagged by **both** lenses (YDF + "
               "cascade) — single-lens signals are quarantined to "
               "`single_lens_signals.tsv`.")
    out.append("")

    for criterion in criteria:
        out.append(f"### Criterion: `{criterion}`")
        out.append("")
        for mech in closed_mechs:
            cell_gaps = cells.get((criterion, mech), [])
            out.append(f"#### Mechanism: `{mech}`")
            out.append("")
            if (criterion, mech) in DEMOTED_CELLS:
                n_demoted = len(cell_gaps)
                out.append(
                    f"> **demoted** by ac-12 attestation (2026-05-23) — "
                    f"{n_demoted} clusters routed to "
                    f"`single_lens_signals.tsv`. See "
                    f"`spot_check_prescreen.md` for the per-gap reasoning "
                    f"and `progress.md` (2026-05-23 entry) for the "
                    f"demotion rationale."
                )
                out.append("")
                continue
            if not cell_gaps:
                out.append("> no corroborated gaps found")
                out.append("")
                continue
            top = cell_gaps[:PART1_TOPN_PER_CELL]
            total_clusters = len(cell_gaps)
            total_cols = sum(g["affected_column_count"] for g in cell_gaps)
            top_cols = sum(g["affected_column_count"] for g in top)
            out.append(
                f"Total clusters in cell: **{total_clusters}** "
                f"({total_cols} distinct columns affected). "
                f"Top-{len(top)} below cover {top_cols} columns "
                f"({top_cols / max(1, total_cols) * 100:.1f}% of cell)."
            )
            out.append("")
            for g in top:
                rec_action = g["recommended_action_class"]
                gid = g["gap_id"]
                rank = g["rank_within_cell"]
                affected = g["affected_column_count"]
                lenses = g["corroborating_lenses"] or []
                lens_str = " · ".join(
                    f"**{l['lens_name']}** = `{l['prediction_or_annotation']}`"
                    f" (conf {l['confidence']:.2f})"
                    for l in lenses
                )
                slug = g.get("candidate_spec_slug") or ""

                out.append(
                    f"##### Rank #{rank} — `{gid[:12]}…` — "
                    f"{affected} columns — action: `{rec_action}`"
                )
                out.append("")
                out.append(f"- **Corroborating lenses**: {lens_str}")
                out.append(
                    f"- **Candidate spec slug**: `{slug}`"
                    if slug else "- **Candidate spec slug**: _(none — to be assigned downstream)_"
                )
                out.append("")
                out.append("| file | column | sense | ydf | samples | dbpedia |")
                out.append("|---|---|---|---|---|---|")
                for s in (g["sample_evidence"] or [])[:5]:
                    fp = s["file_path"]
                    cn = s["column_name"]
                    sense = s.get("sense_prediction") or ""
                    ydf = s.get("ydf_prediction") or ""
                    samples_md = fmt_samples(s.get("sample_values") or [])
                    dbp = dbp_lookup.get((fp, cn), "")
                    # Shorten file path for display: last 2 path components
                    parts = fp.split("/")
                    short_fp = "/".join(parts[-2:]) if len(parts) >= 2 else fp
                    out.append(
                        f"| `{short_fp}` | `{cn}` | `{sense}` | `{ydf}` "
                        f"| {samples_md} | `{dbp}` |"
                    )
                out.append("")
    return "\n".join(out)


def build_part2(
    dbpedia_annotations_path: Path,
    dbpedia_mapping_path: Path,
    files_path: Path,
    columns_path: Path,
) -> str:
    """DBpedia no_finetype_equivalent classes appearing in trivial-predicted
    columns of criterion-(a)-failing files."""
    import duckdb  # type: ignore

    con = duckdb.connect()

    # Load the no_finetype_equivalent classes
    nf_rows = con.execute(f"""
        SELECT dbpedia_or_schemaorg_class AS dbpedia_class
        FROM read_csv('{dbpedia_mapping_path}', sep='\t', header=true)
        WHERE mapping_status = 'no_finetype_equivalent'
    """).fetchall()
    no_fit_classes = {r[0] for r in nf_rows}

    # criterion-a-failing files
    crit_a_files_table = con.execute(f"""
        CREATE TEMP VIEW crit_a_files AS
        SELECT file_path
        FROM read_parquet('{files_path}')
        WHERE error IS NULL
          AND n_cols > 0
          AND CAST(non_trivial_cols AS DOUBLE) / n_cols < {NON_TRIVIAL_FLOOR}
    """)

    # Trivial-predicted columns within criterion-a-failing files,
    # joined with DBpedia annotations. Group by dbpedia_class.
    nf_classes_sql = ", ".join(
        "'" + c.replace("'", "''") + "'" for c in sorted(no_fit_classes)
    )
    if not no_fit_classes:
        return ("## Part 2 — Candidate taxonomy gaps from DBpedia coverage\n\n"
                "> no `no_finetype_equivalent` rows in mapping table; "
                "section empty.\n")

    rows = con.execute(f"""
        WITH triv AS (
          SELECT c.file_path, c.column_name, c.sense_prediction
          FROM read_parquet('{columns_path}') c
          JOIN crit_a_files USING (file_path)
          WHERE c.is_trivial = true
        ),
        joined AS (
          SELECT
            a.dbpedia_semantic_class AS dbpedia_class,
            t.file_path, t.column_name
          FROM read_parquet('{dbpedia_annotations_path}') a
          JOIN triv t
            ON a.file_path = t.file_path
           AND a.column_name = t.column_name
          WHERE a.dbpedia_semantic_class IN ({nf_classes_sql})
        )
        SELECT
          dbpedia_class,
          COUNT(*) AS affected_column_count,
          ANY_VALUE(file_path) AS sample_f1,
          ANY_VALUE(column_name) AS sample_c1
        FROM joined
        GROUP BY 1
        HAVING affected_column_count >= {PART2_MIN_COLS}
        ORDER BY affected_column_count DESC, dbpedia_class ASC
    """).fetchall()

    # For each surfaced class, pull 3 sample (file, column) pairs
    samples_per_class: dict[str, list[tuple[str, str]]] = {}
    for r in rows:
        dbp_class = r[0]
        sample_rows = con.execute(f"""
          SELECT a.file_path, a.column_name
          FROM read_parquet('{dbpedia_annotations_path}') a
          JOIN (
            SELECT c.file_path, c.column_name
            FROM read_parquet('{columns_path}') c
            JOIN crit_a_files USING (file_path)
            WHERE c.is_trivial = true
          ) t
          ON a.file_path = t.file_path AND a.column_name = t.column_name
          WHERE a.dbpedia_semantic_class = '{dbp_class.replace("'", "''")}'
          ORDER BY a.file_path, a.column_name
          LIMIT {PART2_SAMPLE_COLS}
        """).fetchall()
        samples_per_class[dbp_class] = sample_rows

    out: list[str] = []
    out.append("## Part 2 — Candidate taxonomy gaps from DBpedia coverage")
    out.append("")
    out.append(
        "DBpedia classes flagged as `no_finetype_equivalent` in the curated "
        f"mapping table, surfaced when they appear in ≥{PART2_MIN_COLS} "
        "columns AND those columns were predicted as trivial "
        "(`plain_text` / `decimal_number`) by Sense. These are candidate "
        "FineType taxonomy additions — real-world semantic patterns that "
        "DBpedia models but FineType doesn't. **Part 2 is independent of "
        "Part 1** — entries here are not corroborated by the lens stack."
    )
    out.append("")
    out.append("| DBpedia class | columns affected | sample (file:column) |")
    out.append("|---|---:|---|")
    for r in rows:
        dbp_class = r[0]
        cnt = r[1]
        samples = samples_per_class.get(dbp_class, [])
        sample_md = "<br>".join(
            f"`{Path(f).name}:{c}`" for f, c in samples
        )
        out.append(f"| `{dbp_class}` | {cnt} | {sample_md} |")
    out.append("")
    out.append(f"Total `no_finetype_equivalent` classes surfaced: **{len(rows)}**.")
    return "\n".join(out)


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--out", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/report.md")
    p.add_argument("--corroborated", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/corroborated_gaps.parquet")
    p.add_argument("--dbpedia-annotations", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/dbpedia_annotations.parquet")
    p.add_argument("--dbpedia-mapping", type=Path,
                   default=REPO / "eval/gittables/dbpedia_finetype_mapping.tsv")
    p.add_argument("--files-parquet", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/files.parquet")
    p.add_argument("--columns-parquet", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/columns.parquet")
    p.add_argument("--model-dir", type=Path,
                   default=REPO / "models/default")
    p.add_argument("--ydf-path", type=Path,
                   default=REPO / "eval/gittables/models/ydf.bin")
    p.add_argument("--corpus-index-sha", type=Path,
                   default=REPO / "eval/gittables/corpus_paths.sha256")
    args = p.parse_args()

    try:
        import duckdb  # noqa: F401 -- imported by the build_part* helpers
    except ImportError as exc:
        print(f"error: duckdb missing ({exc})", file=sys.stderr)
        return 2

    print("computing frontmatter SHAs...", file=sys.stderr)
    shas = compute_frontmatter_shas(
        args.model_dir, args.ydf_path,
        args.dbpedia_mapping, args.corpus_index_sha,
    )

    print("building Part 1...", file=sys.stderr)
    part1 = build_part1(args.corroborated, args.dbpedia_annotations)
    print("building Part 2...", file=sys.stderr)
    part2 = build_part2(
        args.dbpedia_annotations, args.dbpedia_mapping,
        args.files_parquet, args.columns_parquet,
    )

    # Compose final document
    out: list[str] = []
    out.append("---")
    for k in ("model_sha", "ydf_sha", "dbpedia_mapping_sha",
              "cascade_version", "corpus_index_sha", "corpus_pass_id"):
        out.append(f"{k}: {shas[k]}")
    out.append("---")
    out.append("")
    out.append("# Gittables multi-lens corpus diagnostic — report")
    out.append("")
    out.append(
        "This report is the deliverable of "
        "`.orbit/specs/2026-05-20-gittables-multi-lens-diagnostic/`. "
        "It surfaces and ranks; it does not fix. Runtime metadata "
        "(timings, error counts, version provenance beyond the "
        "frontmatter) lives in `progress.md` — by design, this "
        "document carries no wall-clock timestamp so byte-identical "
        "re-runs are detectable via `corpus_pass_id`."
    )
    out.append("")
    out.append(part1)
    out.append("")
    out.append(part2)
    out.append("")

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("\n".join(out))
    print(json.dumps({
        "output": str(args.out),
        "corpus_pass_id": shas["corpus_pass_id"],
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

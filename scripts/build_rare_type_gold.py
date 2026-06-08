#!/usr/bin/env python3
"""Pre-promotion scoreboard for the contested, header-identifiable rare types
the corpus precision metric is blind to (finding: output/eval-ceiling-diagnosis/).

These types are header-identifiable but value-ambiguous — a latitude column and
an RMS-error column have the same values; only the header separates them. So the
HEADER is the trusted label, validated against the values. No gated-YDF oracle.

Per type we score each round's `sense_prediction`:
  recall  = positives (genuine T columns) the model calls T
  FP-rate = hard-negatives (clearly not-T, by header) the model wrongly calls T
FP-rate is the cross-round comparator — it moves when a round relocates errors,
exactly what aggregate corpus precision (~0.49) cannot register.

  latitude : pos = lat header + decimals in [-90,90];  neg = numeric non-coord
  url      : pos = url header + http/www values;        neg = non-url text columns
  utc      : pos = none (utc-offset ~absent in corpus); neg = integer quantities
             (the real battle is utc-on-integers; FP-rate only)

Usage:  python3 scripts/build_rare_type_gold.py
"""
import subprocess, re

ANSI = re.compile(r"\x1b\[[0-9;]*m")
SEP = "chr(9474)"  # U+2502, the sample-value delimiter

MODELS = [
    ("v19 (shipped)", "output/ydf-validation-gate/v19_gated.parquet"),
    ("v22",           "output/ydf-validation-gate/v22_gated.parquet"),
    ("v23",           "output/ydf-validation-gate/v23_gated.parquet"),
    ("latdec",        "eval/gittables/corpus_pass_latdec/corpus_pass/columns.parquet"),
    ("v0624",         "output/corpus-honest-gate/v0624_pass/corpus_pass/columns.parquet"),
    ("fusion_v27",    "output/corpus-honest-gate/fusion_v27_pass/corpus_pass/columns.parquet"),
]

LAT_POS = r'(x|y|geo|dec|decimal|wgs84)?[ _-]?lat(itude)?[ _-]?(dd|deg|degrees|decimal|n|s|wgs84|coord|gms|new|x|y|[0-9])?'
LAT_NEG = (r'(population|score|rating|\brate\b|ratio|error|\berr\b|rms|residual|magnitude|\bmag\b|'
           r'depth|temperature|\btemp\b|price|cost|amount|weight|height|\bage\b|percent|\bpct\b|'
           r'probability|\bprob\b|correlation|coefficient|\bcoef\b|\bmean\b|median|stdev|\bstd\b|'
           r'variance|\bvar\b|salary|income|revenue|\bgdp\b|density|frequency|voltage|velocity|'
           r'pressure|balance|elevation|distance|speed)')
URL_POS = r'(url|uri|\blink\b|href|website|web_?site|homepage|web_?page|page_?url|image_?url|img_?url|source_?url|profile_?url|permalink|\bdomain\b)'
URL_NEG = (r'(^name$|first_?name|last_?name|full_?name|\btitle\b|description|comment|author|publisher|'
           r'category|^status$|^type$|^label$|gender|colou?r|\bbrand\b|^city$|^country$|^state$|'
           r'company|organi[sz]ation)')
UTC_NEG = (r'(^id$|_id$|count|^num$|number|index|^rank$|\bqty\b|quantity|population|votes|views|'
           r'\bsize\b|amount|price|score|^total$|^sum$|height|weight)')

# type -> (target label, pos-SQL or None, neg-SQL).  SQL predicates reference the
# value-feature columns computed in `vs`.
TYPES = {
    "latitude": (
        "geography.coordinate.latitude",
        f"regexp_full_match(h, '{LAT_POS}') AND n>=5 AND n_num>=n*0.9 AND n_latlike>=n_num*0.8",
        f"regexp_matches(h, '{LAT_NEG}') AND n>=5 AND n_num>=n*0.9",
    ),
    "url": (
        "technology.internet.url",
        f"regexp_matches(h, '{URL_POS}') AND n>=5 AND n_url>=n*0.7",
        f"regexp_matches(h, '{URL_NEG}') AND n>=5 AND n_str>=n*0.8 AND n_url<=n*0.05",
    ),
    "utc": (
        "datetime.offset.utc",
        None,
        f"regexp_matches(h, '{UTC_NEG}') AND n>=5 AND n_int>=n*0.9",
    ),
}

def sql(parquet):
    branches = []
    for typ, (label, pos, neg) in TYPES.items():
        pos_when = f"WHEN {pos} THEN 'pos'" if pos else ""
        branches.append(f"""
SELECT '{typ}' typ,
  CASE {pos_when} WHEN {neg} THEN 'neg' ELSE NULL END grole,
  (sp = '{label}')::INT hit
FROM vs""")
    union = "\nUNION ALL".join(branches)
    return f"""
WITH base AS (
  SELECT lower(trim(column_name)) h, sense_prediction sp,
         string_split(sample_values_truncated, {SEP}) vals
  FROM '{parquet}'
  WHERE sample_values_truncated IS NOT NULL AND sample_values_truncated <> ''
),
vs AS (
  SELECT h, sp, len(vals) n,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) IS NOT NULL)) n_num,
    len(list_filter(vals, v -> TRY_CAST(v AS BIGINT) IS NOT NULL)) n_int,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) BETWEEN -90 AND 90 AND contains(v,'.'))) n_latlike,
    len(list_filter(vals, v -> regexp_matches(lower(v), '^\\s*(https?://|www\\.)'))) n_url,
    len(list_filter(vals, v -> TRY_CAST(v AS DOUBLE) IS NULL AND length(trim(v))>0)) n_str
  FROM base
)
SELECT typ, grole, count(*) cols, sum(hit) preds
FROM ({union})
WHERE grole IS NOT NULL
GROUP BY typ, grole ORDER BY typ, grole;
"""

def run(parquet):
    out = subprocess.run(["duckdb", "-init", "/dev/null", "-noheader", "-list",
                          "-c", sql(parquet)], capture_output=True, text=True)
    d = {}  # (typ, role) -> (cols, preds)
    for line in out.stdout.splitlines():
        p = [x.strip() for x in ANSI.sub("", line).strip().split("|")]
        if len(p) == 4 and p[2].isdigit():
            d[(p[0], p[1])] = (int(p[2]), int(p[3]))
    return d

def main():
    data = {label: run(pq) for label, pq in MODELS}
    for typ in TYPES:
        has_pos = TYPES[typ][1] is not None
        print(f"\n=== {typ} ({TYPES[typ][0]}) ===")
        head = f"{'model':<16}" + (f"{'pos':>7}{'recall':>8}" if has_pos else "") + \
               f"{'neg':>9}{'FP-rate':>9}"
        print(head); print("-" * len(head))
        for label, _ in MODELS:
            d = data[label]
            line = f"{label:<16}"
            if has_pos:
                npos, tp = d.get((typ, "pos"), (0, 0))
                line += f"{npos:>7}" + (f"{tp/npos:>8.3f}" if npos else f"{'—':>8}")
            nneg, fp = d.get((typ, "neg"), (0, 0))
            line += f"{nneg:>9}" + (f"{fp/nneg:>9.4f}" if nneg else f"{'—':>9}")
            print(line)
    # Compact pre-promotion scoreboard: the one comparable number per type.
    print(f"\n=== SCOREBOARD (corpus precision is ~0.49 for every row) ===")
    hdr = f"{'model':<16}{'lat FP-rate↓':>14}{'url recall↑':>13}{'utc FP-rate↓':>14}"
    print(hdr); print("-" * len(hdr))
    for label, _ in MODELS:
        d = data[label]
        lneg, lfp = d.get(("latitude", "neg"), (0, 0))
        upos, utp = d.get(("url", "pos"), (0, 0))
        uneg, ufp = d.get(("utc", "neg"), (0, 0))
        print(f"{label:<16}"
              f"{(lfp/lneg if lneg else 0):>14.4f}"
              f"{(utp/upos if upos else 0):>13.3f}"
              f"{(ufp/uneg if uneg else 0):>14.4f}")
    print("\nFP-rate = clearly-not-T columns (by header) the model wrongly calls T;")
    print("recall = genuine-T columns the model gets. These move per round where")
    print("aggregate corpus precision (~0.49) does not.")


if __name__ == "__main__":
    main()

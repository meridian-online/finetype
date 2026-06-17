#!/usr/bin/env python3
"""ac-02 gold migration (spec 2026-06-17-enum-accuracy-reframe): relabel
`representation.discrete.categorical` -> text residual / adjudicated type.

LINE-SURGICAL: the gold TSV is plain tab-delimited with NO quoting and one
logical row per physical line, so we edit only the curated_label + provenance
fields of the 95 target lines and leave every other line BYTE-IDENTICAL (a csv
round-trip would reformat all 932 lines and obscure the change).

Adjudication outcomes are keyed on (column_name, value-tuple) from the
off-categorical panel — robust to row order and duplicate column names.
One-shot, already applied; kept as the audit record. Reversible via git +
the append-only provenance string.
"""
import csv, json, subprocess
from pathlib import Path

GOLD = Path("eval/gold/gold_corpus.tsv")
CAT = "representation.discrete.categorical"
SPEC = "spec 2026-06-17-enum-accuracy-reframe"
WORD = "representation.text.word"

# (column_name, values-tuple) -> new label, from the off-categorical panel
recat = json.load(open("/tmp/recat_input.json"))
outc = json.load(open(str(Path(__file__).parent.parent /
                       ".orbit/specs/2026-06-17-enum-accuracy-reframe/ac02-residual-labels.json")))
adj = {(r["column_name"], tuple(r["values"])): outc[str(r["idx"])]
       for r in recat if str(r["idx"]) in outc}

# Preserve CRLF line endings exactly (the fixture is CRLF; read_text() would strip \r
# off all 932 lines and show the whole file as changed).
raw = GOLD.read_bytes().decode("utf-8").split("\r\n")
header = raw[0].split("\t")
i_label = header.index("curated_label")
i_sha = header.index("file_content_sha256")
i_col = header.index("column_name")
i_prov = header.index("provenance")

# corpus values for the categorical rows, to resolve the (col, values) key
cat_keys = []
for line in raw[1:]:
    if not line:
        continue
    f = line.split("\t")
    if f[i_label] == CAT:
        cat_keys.append((f[i_sha], f[i_col]))
kf = "/tmp/mig_keys.csv"
with open(kf, "w", newline="") as fh:
    w = csv.writer(fh); w.writerow(["sha", "col"])
    w.writerows(cat_keys)
q = ("SELECT k.sha, k.col, c.sample_values_truncated AS sv "
     "FROM read_csv_auto('/tmp/mig_keys.csv', header=true) k "
     "LEFT JOIN 'eval/gittables/corpus_pass/columns.parquet' c "
     "ON c.file_content_sha256=k.sha AND c.column_name=k.col")
data = json.loads(subprocess.run(["duckdb", "-json", "-c", q],
                                 capture_output=True, text=True).stdout or "[]")
vals = {(d["sha"], d["col"]): tuple((d["sv"] or "").split("│")[:12])
        for d in data if d.get("sv")}

out, n_adj, n_def, changes = [raw[0]], 0, 0, {}
for line in raw[1:]:
    if not line:
        out.append(line); continue
    f = line.split("\t")
    if f[i_label] != CAT:
        out.append(line); continue
    k = (f[i_sha], f[i_col]); vt = vals.get(k)
    if vt is not None and (f[i_col], vt) in adj:
        new = adj[(f[i_col], vt)]; n_adj += 1; how = "adjudicated"
    else:
        new = WORD; n_def += 1; how = "residual default (no corpus values)"
    assert "\t" not in new
    f[i_label] = new
    f[i_prov] = f[i_prov] + f"; {SPEC}: was {CAT} (enum reframe; {how})"
    out.append("\t".join(f))
    changes[new] = changes.get(new, 0) + 1

GOLD.write_bytes(("\r\n".join(out)).encode("utf-8"))
print(f"migrated {n_adj + n_def} rows (adjudicated={n_adj}, default={n_def})")
print("new labels:", dict(sorted(changes.items(), key=lambda x: -x[1])))

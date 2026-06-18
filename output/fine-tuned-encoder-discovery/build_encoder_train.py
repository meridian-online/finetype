"""ac-01 training-data assembly: build header+values labelled columns for the contested
escalation arbiter, from authoritative vocabularies (clean positives) + mined over-emissions
and distilled text (residual negatives).

Families = the contested decision: RESIDUAL + country/country_code/city/region/full_name/
entity_name/iata_code. Output: encoder_train.tsv (text, label). Disjoint from gold (vocabs +
distilled-sherlock + corpus-mined; gold is the held-out test).
"""
import csv, gzip, json, random
random.seed(42)
GN = "/Users/hugh/datasets/geonames/2026-05-24/"
LR = "eval/gold/lens_reference/"
CSV = "eval/datasets/csv/"

def load_lines(path, col, sep="\t", enc="utf-8"):
    out = []
    for line in open(path, encoding=enc):
        p = line.rstrip("\n").split(sep)
        if len(p) > col and p[col].strip():
            out.append(p[col].strip())
    return out

# --- authoritative vocabularies (clean positives) ---
cities = load_lines(LR + "cities15000.txt", 1) + load_lines(LR + "cities15000.txt", 2)
regions = (load_lines(LR + "admin1CodesASCII.txt", 1) + load_lines(LR + "admin1CodesASCII.txt", 2)
           + load_lines(GN + "admin2Codes.txt", 1))
for r in csv.reader(open(CSV + "us_states.csv")):
    regions += [x for x in r if x]
countries = [r["name"] for r in csv.DictReader(open(LR + "iso3166.csv"))]
ccodes = []
for r in csv.DictReader(open(LR + "iso3166.csv")):
    ccodes += [r["alpha-2"], r["alpha-3"]]
# airports.csv -> iata codes (3-letter)
iata = []
for r in csv.DictReader(open(CSV + "airports.csv")):
    for k, v in r.items():
        if "iata" in k.lower() and v and len(v.strip()) == 3:
            iata.append(v.strip())
# people_directory -> full names (+ synth first+last)
fns, lns, fulls = [], [], []
for r in csv.DictReader(open(CSV + "people_directory.csv")):
    if r.get("full_name"): fulls.append(r["full_name"])
    if r.get("first_name"): fns.append(r["first_name"])
    if r.get("last_name"): lns.append(r["last_name"])
for _ in range(4000):
    if fns and lns: fulls.append(f"{random.choice(fns)} {random.choice(lns)}")

VOCAB = {
    "city": [v for v in cities if v], "region": [v for v in regions if v],
    "country": countries, "country_code": [c for c in ccodes if c],
    "iata_code": iata or ["JFK", "LHR", "CDG", "NRT", "SFO", "DXB"], "full_name": fulls,
}
HEADERS = {
    "city": ["city", "city_name", "town", "municipality", "place", "locality"],
    "region": ["state", "region", "province", "county", "district", "admin_region", "us_state"],
    "country": ["country", "country_name", "nation", "land"],
    "country_code": ["country_code", "country_iso", "iso2", "cc", "code"],
    "iata_code": ["airport", "iata", "airport_code", "code", "origin", "dest"],
    "full_name": ["name", "full_name", "author", "contact", "person", "owner", "applicant"],
}
GENERIC_HEADERS = ["value", "data", "field", "name", "label", "item", "category", "type",
                   "status", "code", "description", "notes", "tag", "key", "col", "attribute"]

def make_col(header, vals, k=8):
    s = random.sample(vals, min(k, len(vals)))
    return f"header: {header} | values: " + ", ".join(s)

rows = []  # (text, label)
PER_FAM = 3000
for fam, vals in VOCAB.items():
    if not vals: continue
    for _ in range(PER_FAM):
        rows.append((make_col(random.choice(HEADERS[fam]), vals), fam))

# --- residual + entity_name from distilled (header-less -> generic headers) ---
RESID_SRC = {"representation.discrete.categorical", "representation.text.word", "representation.text.plain_text"}
dist_resid, dist_entity = [], []
with gzip.open("output/distillation-v3/sherlock_distilled.csv.gz", "rt") as f:
    for r in csv.DictReader(f):
        lab = r.get("final_label") or ""; sv = r.get("sample_values") or ""
        if not sv: continue
        try: vals = [str(v) for v in json.loads(sv) if str(v).strip() and str(v) != "None"]
        except Exception: continue
        if not vals: continue
        if lab in RESID_SRC: dist_resid.append(vals)
        elif lab == "representation.text.entity_name": dist_entity.append(vals)
for vals in dist_resid[:6000]:
    rows.append((make_col(random.choice(GENERIC_HEADERS), vals), "RESIDUAL"))
for vals in dist_entity[:3000]:
    rows.append((make_col(random.choice(GENERIC_HEADERS + ["company", "organization", "brand", "product", "title"]), vals), "entity_name"))

# --- residual from mined corpus over-emissions (geo/person preds failing vocab membership) ---
def norm(s): return s.strip().lower()
SEP = "│"
vocsets = {k: set(norm(x) for x in v) for k, v in VOCAB.items()}
mined_resid = 0
for r in csv.DictReader(open("output/fine-tuned-encoder-discovery/mining_pool.tsv"), delimiter="\t"):
    leaf, hdr, sv = r["leaf"], r["column_name"], r["sample_values_truncated"]
    vals = [v.strip() for v in (sv or "").split(SEP) if v.strip()][:8]
    if not vals: continue
    if leaf in vocsets:                       # a geo prediction
        frac = sum(norm(v) in vocsets[leaf] for v in vals) / len(vals)
        if frac < 0.4:                        # values aren't really that type -> residual
            rows.append((f"header: {hdr} | values: " + ", ".join(vals), "RESIDUAL")); mined_resid += 1

random.shuffle(rows)
with open("output/fine-tuned-encoder-discovery/encoder_train.tsv", "w", newline="") as f:
    w = csv.writer(f, delimiter="\t", lineterminator="\n")
    w.writerow(["text", "label"])
    for t, l in rows: w.writerow([t.replace("\t", " ").replace("\n", " "), l])
from collections import Counter
print(f"vocab sizes: city={len(VOCAB['city'])} region={len(VOCAB['region'])} country={len(VOCAB['country'])} "
      f"cc={len(VOCAB['country_code'])} iata={len(VOCAB['iata_code'])} full_name={len(VOCAB['full_name'])}")
print(f"mined over-emission residuals added: {mined_resid}")
print(f"total train rows {len(rows)}; label dist: {dict(Counter(l for _, l in rows))}")

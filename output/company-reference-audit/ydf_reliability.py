#!/usr/bin/env python3
"""Measure gated-YDF's validation reliability per checksum type.

For each checksum-bearing label L that gated-YDF asserts (ydf_prediction_gated=L),
what fraction of those columns actually carry a valid check digit? A reliable
oracle asserts a type only for real instances; a shape-matcher asserts it for
anything the right length. Ground truth = the real check-digit algorithm.
"""
import duckdb, sys, statistics

CAND = sys.argv[1]   # candidate corpus pass (has sample_values_truncated)
BASE = sys.argv[2]   # baseline with ydf_prediction_gated
DELIM = "│"      # box-drawing vertical used in sample_values_truncated

# ---- check-digit algorithms (match crates/finetype-core/src/checksum.rs) ----
def _digits(s, strip=" -"):
    t = s.strip()
    if t[:1] in "+-": return None
    for c in strip: t = t.replace(c, "")
    return t if t.isdigit() else None

def luhn(s):
    d = _digits(s)
    if d is None or len(d) < 2: return False
    tot = 0
    for i, ch in enumerate(reversed(d)):
        x = int(ch)
        if i % 2 == 1:
            x *= 2
            if x > 9: x -= 9
        tot += x
    return tot % 10 == 0

def npi(s):
    d = _digits(s)
    if d is None or len(d) != 10: return False
    return luhn("80840" + d)

def gs1(s):
    d = _digits(s)
    if d is None or len(d) not in (8, 12, 13, 14): return False
    tot = 0
    for i, ch in enumerate(reversed(d)):
        tot += int(ch) * (3 if i % 2 == 1 else 1)
    return tot % 10 == 0

def isbn(s):
    t = s.strip()
    if t[:1] in "+-": return False
    d = t.replace("-", "")
    if len(d) == 10:
        tot = 0
        for i, ch in enumerate(d):
            v = 10 if (i == 9 and ch in "Xx") else (int(ch) if ch.isdigit() else None)
            if v is None: return False
            tot += (i + 1) * v
        return tot % 11 == 0
    if len(d) == 13 and d.isdigit():
        tot = sum((int(ch) if i % 2 == 0 else 3 * int(ch)) for i, ch in enumerate(d))
        return tot % 10 == 0
    return False

def aba(s):
    d = _digits(s)
    if d is None or len(d) != 9: return False
    w = [3,7,1,3,7,1,3,7,1]
    return sum(w[i]*int(c) for i,c in enumerate(d)) % 10 == 0

def _av(c):  # alnum36
    if c.isdigit(): return int(c)
    if c.isupper() and c.isalpha(): return ord(c) - ord('A') + 10
    return None

def isin(s):
    ch = [c for c in s.strip() if c not in " -"]
    if len(ch) != 12: return False
    exp = ""
    for c in ch:
        v = _av(c)
        if v is None: return False
        exp += str(v)
    return luhn(exp)

def mod97(ch):
    r = 0
    for c in ch:
        v = _av(c)
        if v is None: return None
        if v >= 10:
            r = (r*10 + v//10) % 97; r = (r*10 + v%10) % 97
        else:
            r = (r*10 + v) % 97
    return r

def lei(s):
    ch = [c for c in s.strip() if c != " "]
    if len(ch) != 20: return False
    return mod97(ch) == 1

def iban(s):
    ch = [c.upper() for c in s.strip() if c != " "]
    if not (15 <= len(ch) <= 34): return False
    if not (ch[0].isalpha() and ch[1].isalpha() and ch[2].isdigit() and ch[3].isdigit()): return False
    return mod97(ch[4:] + ch[:4]) == 1

def cusip(s):
    ch = list(s.strip())
    if len(ch) != 9: return False
    tot = 0
    def cv(c):
        if c.isdigit(): return int(c)
        if c.isupper() and c.isalpha(): return ord(c)-ord('A')+10
        return {'*':36,'@':37,'#':38}.get(c)
    for i,c in enumerate(ch[:8]):
        v = cv(c)
        if v is None: return False
        if i%2==1: v*=2
        tot += v//10 + v%10
    if not ch[8].isdigit(): return False
    return int(ch[8]) == (10 - tot%10)%10

def sedol(s):
    ch = list(s.strip())
    if len(ch) != 7: return False
    w=[1,3,1,7,3,9]; tot=0
    for i,c in enumerate(ch[:6]):
        v=_av(c)
        if v is None: return False
        tot += w[i]*v
    if not ch[6].isdigit(): return False
    return int(ch[6]) == (10 - tot%10)%10

def figi(s):
    ch=[c.upper() for c in s.strip()]
    if len(ch)!=12 or not ch[11].isdigit(): return False
    tot=0
    for i,c in enumerate(ch[:11]):
        v=_av(c)
        if v is None: return False
        if i%2==1: v*=2
        tot += v//10 + v%10
    return int(ch[11]) == (10 - tot%10)%10

CHECKS = {
    "identity.medical.npi": npi,
    "identity.commerce.upc": gs1,
    "identity.commerce.ean": gs1,
    "finance.payment.credit_card_number": luhn,
    "identity.commerce.isbn": isbn,
    "finance.banking.aba_routing": aba,
    "finance.securities.isin": isin,
    "finance.securities.cusip": cusip,
    "finance.securities.sedol": sedol,
    "finance.securities.figi": figi,
    "finance.securities.lei": lei,
    "finance.banking.iban": iban,
}

# ---- pull oracle-asserted columns with their sample values ----
rows = duckdb.sql(f"""
    SELECT b.ydf_prediction_gated AS label, c.sample_values_truncated AS vals
    FROM read_parquet('{BASE}') b
    JOIN read_parquet('{CAND}') c USING (file_path, column_name)
    WHERE b.ydf_prediction_gated IN ({','.join(f"'{k}'" for k in CHECKS)})
      AND c.sample_values_truncated IS NOT NULL
""").fetchall()

from collections import defaultdict
bylabel = defaultdict(list)
for label, vals in rows:
    bylabel[label].append(vals)

print(f"{'oracle asserts type':<38} {'cols':>6} {'median':>7} {'>=50%':>7} {'<10%':>6}  read as")
print("-"*88)
out = []
for label, fn in CHECKS.items():
    cols = bylabel.get(label, [])
    if not cols: continue
    passrates = []
    for vals in cols:
        vs = [v for v in vals.split(DELIM) if v.strip()]
        if not vs: continue
        passrates.append(sum(1 for v in vs if fn(v)) / len(vs))
    if not passrates: continue
    n = len(passrates)
    med = statistics.median(passrates)
    genuine = sum(1 for p in passrates if p >= 0.5) / n
    shape = sum(1 for p in passrates if p < 0.1) / n
    verdict = "RELIABLE" if genuine >= 0.7 else ("SHAPE-MATCH" if genuine <= 0.2 else "mixed")
    out.append((label, n, med, genuine, shape, verdict))

# sort by reliability ascending (worst first)
for label, n, med, genuine, shape, verdict in sorted(out, key=lambda r: r[3]):
    print(f"{label:<38} {n:>6} {med:>7.2f} {genuine:>6.1%} {shape:>5.0%}  {verdict}")

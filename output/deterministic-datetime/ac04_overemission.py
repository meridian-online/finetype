"""ac-04 corpus over-emission sanity check (directional, non-blocking).

A/B the datetime_format_refinement rule at corpus scale using ONE instrumented binary:
rule ON (default) vs rule OFF (RHH_DISABLE_HINTS=datetime_format_refinement). Same model,
same Sharpen chain, same sibling context — the ONLY difference is our rule, so every label
delta is attributable to it exactly (no confounds). Single-column CSVs match the gold
predict methodology.

Question: does the rule pull NON-datetime mass INTO datetime, or explode a datetime
sub-leaf marginal? Reports per-leaf marginal shift + categorises every transition.
"""
import csv, subprocess, sys, json, random, tempfile, os
import pyarrow.parquet as pq

SEP = "│"
BIN = "target/release/finetype"
RULE = "datetime_format_refinement"
N = int(sys.argv[1]) if len(sys.argv) > 1 else 2000
random.seed(13)

def profile(col, vals, rule_on):
    with tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False, newline="") as f:
        w = csv.writer(f); w.writerow([col])
        for v in vals: w.writerow([v])
        path = f.name
    env = dict(os.environ)
    if not rule_on:
        env["RHH_DISABLE_HINTS"] = RULE
    try:
        p = subprocess.run([BIN, "profile", "-f", path, "-o", "json-schema"],
                           capture_output=True, text=True, env=env, timeout=30)
        os.unlink(path)
        if p.returncode != 0: return None
        d = json.loads(p.stdout)
        props = d.get("properties", d)
        k = next(iter(props))
        return props[k].get("x-finetype-label", "")
    except Exception:
        try: os.unlink(path)
        except OSError: pass
        return None

# Sample N random non-trivial corpus columns.
tbl = pq.read_table("eval/gittables/corpus_pass/columns.parquet",
                    columns=["column_name", "sample_values_truncated", "is_trivial"])
rows = tbl.to_pylist()
pool = [r for r in rows if not r.get("is_trivial") and (r.get("sample_values_truncated") or "")]
random.shuffle(pool)
sample = pool[:N]
print(f"profiling {len(sample)} columns x2 (rule on/off)...", file=sys.stderr)

from collections import Counter
on_marg, off_marg = Counter(), Counter()
into_dt, out_dt, subleaf, changed = [], [], [], 0
for i, r in enumerate(sample, 1):
    col = r["column_name"] or "col"
    vals = [v for v in (r["sample_values_truncated"] or "").split(SEP) if v][:40]
    if not vals: continue
    on = profile(col, vals, True)
    off = profile(col, vals, False)
    if on is None or off is None: continue
    on_marg[on] += 1; off_marg[off] += 1
    if on != off:
        changed += 1
        on_dt = on.startswith("datetime."); off_dt = off.startswith("datetime.")
        if on_dt and not off_dt: into_dt.append((col, off, on, vals[:3]))
        elif off_dt and not on_dt: out_dt.append((col, off, on, vals[:3]))
        elif on_dt and off_dt: subleaf.append((col, off, on))
    if i % 250 == 0: print(f"  ...{i}/{len(sample)}", file=sys.stderr)

print(f"\n=== ac-04 over-emission A/B (n={len(sample)}) ===")
print(f"columns the rule changed: {changed}")
print(f"  INTO datetime (non-dt -> dt): {len(into_dt)}")
print(f"  OUT of datetime (dt -> non-dt): {len(out_dt)}")
print(f"  datetime SUB-LEAF shift: {len(subleaf)}")

print("\n--- per-datetime-leaf marginal (off -> on) ---")
dts = sorted(set(k for k in list(on_marg)+list(off_marg) if k.startswith("datetime.")))
for k in dts:
    d = on_marg[k]-off_marg[k]
    print(f"  {off_marg[k]:4d} -> {on_marg[k]:4d}  ({d:+d})  {k}")

print("\n--- INTO-datetime transitions (verify these are GENUINE timestamps) ---")
for col, off, on, ex in into_dt[:40]:
    print(f"  {col[:24]:24} {off} -> {on}   e.g. {ex}")
print("\n--- OUT-of-datetime transitions (rule should rarely remove datetime) ---")
for col, off, on, ex in out_dt[:20]:
    print(f"  {col[:24]:24} {off} -> {on}   e.g. {ex}")

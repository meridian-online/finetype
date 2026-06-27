#!/usr/bin/env python3
"""Build an FTMB with the embed slot computed from a BIGGER potion (8M/32M), in Python.

spec 2026-06-21 ac-02. The vanilla builder gets its embed slot from `finetype
extract-features` (Rust), whose extract_embedding_aggregation TRUNCATES anything wider
than 128 dims (EMBED_DIM const). So pointing models/model2vec at potion-8M and running
the normal builder would silently train on the first 128 dims — a fake null. This builder
computes the potion embedding aggregation in PYTHON (full width) and patches it into
prepare_multibranch_data, exactly as build_ftmb_v5_gte.py does — a true one-variable
change: the embed encoder only. Every other branch (char/stats/header/validation) and the
whole data blend are v19's, untouched.

Aggregation matches the Rust layout: per value -> model2vec L2-normed vector; per column ->
mean ++ var ++ min ++ max (each potion_dim wide) = 4 x potion_dim. Two-view concatenates
two potions (e.g. 4M ++ 32M), mirroring the gte two-view that carried most of gte's gain.

Usage:
  eval/gittables/.venv/bin/python scripts/build_ftmb_v5_potion.py \
      --potion minishlab/potion-base-8M --output output/multibranch-training/m2v8m-244.ftmb
  # two-view:
  ... --potion minishlab/potion-base-4M --two-view minishlab/potion-base-32M --output ...
  # smoke (format round-trip, no data):  ... --smoke /tmp/potion_smoke.ftmb
"""
import argparse
import struct
import subprocess
import sys

import numpy as np

sys.path.insert(0, "scripts")
import prepare_multibranch_data as P  # noqa: E402

VALUE_CAP = 32  # values encoded per column before aggregation (stats are stable by ~32)

_ENCODERS = []        # loaded StaticModel stack (concat order); each contributes 4*dim
_POTIONS = []         # model names, in concat order
EMBED_AGG_DIM = 0     # set once encoders are loaded


def _load_encoders():
    """Load the potion StaticModel stack once. Single-view: [potion]. Two-view: [a, b]."""
    global _ENCODERS, EMBED_AGG_DIM
    if _ENCODERS:
        return _ENCODERS
    from model2vec import StaticModel
    dims = []
    for name in _POTIONS:
        m = StaticModel.from_pretrained(name)
        d = int(m.embedding.shape[1])
        _ENCODERS.append(m)
        dims.append(d)
        print(f"[build] potion view {len(_ENCODERS)}: {name} (dim {d})")
    EMBED_AGG_DIM = sum(4 * d for d in dims)
    print(f"[build] EMBED_AGG_DIM = {EMBED_AGG_DIM} "
          f"({' ++ '.join(f'4x{d}' for d in dims)})")
    return _ENCODERS


def potion_embed(values):
    """Column embed = per-view 4-stat aggregate over model2vec value vectors, concatenated.
    Matches Rust extract_embedding_aggregation: mean ++ var ++ min ++ max, value vectors
    L2-normed. Empty column -> zeros."""
    encs = _load_encoders()
    texts = [str(v) for v in values[:VALUE_CAP] if v is not None and str(v).strip()]
    if not texts:
        return [0.0] * EMBED_AGG_DIM
    out = []
    for m in encs:
        pv = m.encode(texts, normalize=True)          # (n, dim) L2-normed, matches Rust
        pv = np.asarray(pv, dtype=np.float64)
        out.extend(pv.mean(0))
        out.extend(pv.var(0))                          # population variance (ddof=0)
        out.extend(pv.min(0))
        out.extend(pv.max(0))
    return [float(x) for x in out]


def _probe_valid_dim():
    """Live taxonomy validation width via the binary (so VALID_DIM tracks 244, not stale 240)."""
    out = subprocess.run(
        ["./target/release/finetype", "extract-features", "--json",
         "--header", "probe", "--validation"],
        input='["x"]', capture_output=True, text=True, check=True)
    import json
    return len(json.loads(out.stdout.strip())["validation"])


def install_patches():
    """Patch prepare_multibranch_data: EMBED_DIM->potion agg width, VALID_DIM->live,
    embed slot->potion, version marker 5. Mirrors build_ftmb_v5_gte.install_patches."""
    _load_encoders()
    P.EMBED_DIM = EMBED_AGG_DIM
    live = _probe_valid_dim()
    if live != P.VALID_DIM:
        print(f"[build] VALID_DIM {P.VALID_DIM} -> {live} (live taxonomy)")
        P.VALID_DIM = live
    P.VERSION_V4 = 5  # v5 marker (embed-swapped)

    _orig_extract = getattr(P, "_orig_extract_features", None) or P.extract_features
    P._orig_extract_features = _orig_extract

    def patched_extract_features(finetype_bin, values, header=None, include_validation=False):
        feats = _orig_extract(finetype_bin, values, header=header,
                              include_validation=include_validation)
        if feats is None:
            return None
        feats["embed"] = potion_embed(values)
        return feats

    P.extract_features = patched_extract_features

    _orig_write = getattr(P, "_orig_write_ftmb_v4", None) or P.write_ftmb_v4
    P._orig_write_ftmb_v4 = _orig_write

    def patched_write_ftmb_v4(path, table_groups, valid_dim=None):
        return _orig_write(path, table_groups,
                           valid_dim=P.VALID_DIM if valid_dim is None else valid_dim)

    P.write_ftmb_v4 = patched_write_ftmb_v4


def _verify_header(path):
    with open(path, "rb") as f:
        magic = f.read(4)
        (version,) = struct.unpack("<I", f.read(4))
        (n_records,) = struct.unpack("<Q", f.read(8))
        char_dim, embed_dim, stats_dim, header_dim = struct.unpack("<HHHH", f.read(8))
        (n_groups, _res) = struct.unpack("<HH", f.read(4))
        (valid_dim,) = struct.unpack("<H", f.read(2))
    print(f"[smoke] version={version} n_records={n_records} n_groups={n_groups} "
          f"char={char_dim} embed={embed_dim} stats={stats_dim} header={header_dim} valid={valid_dim}")
    assert magic == b"FTMB", magic
    assert version == 5, version
    assert embed_dim == EMBED_AGG_DIM, (embed_dim, EMBED_AGG_DIM)
    assert valid_dim == P.VALID_DIM, (valid_dim, P.VALID_DIM)


def smoke_write(path):
    """Tiny v5 with synthetic features — proves the format/version independent of data."""
    _load_encoders()
    P.EMBED_DIM = EMBED_AGG_DIM
    P.VERSION_V4 = 5
    P.VALID_DIM = _probe_valid_dim()
    cd, ed, sd, hd, vd = P.CHAR_DIM, EMBED_AGG_DIM, P.STATS_DIM, P.HEADER_DIM, P.VALID_DIM

    def feat(n, base):
        return [float((base + i) % 7) * 0.01 for i in range(n)]

    labels = ["identity.person.email", "geography.location.city",
              "representation.numeric.decimal_number"]
    groups = []
    for g in range(40):
        recs, headers = [], []
        for c in range(3):
            headers.append(f"col_{g}_{c}")
            recs.append({"label": labels[(g + c) % len(labels)], "column_index": c,
                         "char": feat(cd, g * 3 + c), "embed": feat(ed, g + c),
                         "stats": feat(sd, c), "header": feat(hd, g),
                         "validation": feat(vd, g + c)})
        groups.append({"sibling_headers": headers, "records": recs})
    P.write_ftmb_v4(path, groups)
    print(f"[smoke] wrote {path}")
    _verify_header(path)


def smoke_embed():
    """Check the potion embed dimension + that an empty column maps to zeros."""
    _load_encoders()
    v = potion_embed(["jane@x.com", "bob@y.org", "AB12CD34"])
    assert len(v) == EMBED_AGG_DIM, (len(v), EMBED_AGG_DIM)
    z = potion_embed([])
    assert len(z) == EMBED_AGG_DIM and not any(z), "empty column must be zeros"
    nonzero = sum(1 for x in v if abs(x) > 1e-9)
    print(f"[smoke-embed] OK dim={EMBED_AGG_DIM}, nonzero={nonzero}, empty->zeros")


def smoke_v6(path, cap=4):
    """Round-trip proof for FTMB v6 (per-value attention, choice 0106): write a
    tiny v6 with known value strings (unicode + an over-cap column), read back
    via read_ftmb, assert the strings are identical after the cap. Pure format
    test — independent of the potion encoder, the finetype binary, and real data."""
    P.STORE_VALUES_CAP = cap
    cd, ed, sd, hd, vd = P.CHAR_DIM, P.EMBED_DIM, P.STATS_DIM, P.HEADER_DIM, P.VALID_DIM

    def feat(n, base):
        return [float((base + i) % 7) * 0.01 for i in range(n)]

    value_sets = [
        ["jane@x.com", "bob@y.org"],
        ["£42.50", "naïve", "日本語", "x"],
        [],
        ["a", "b", "c", "d", "e", "f"],  # > cap
    ]
    recs = []
    for c in range(4):
        recs.append({"label": "identity.person.email", "column_index": c,
                     "char": feat(cd, c), "embed": feat(ed, c), "stats": feat(sd, c),
                     "header": feat(hd, c), "validation": feat(vd, c),
                     "values": value_sets[c]})
    groups = [{"sibling_headers": [f"col_{c}" for c in range(4)], "records": recs}]
    P.write_ftmb_v6(path, groups, valid_dim=vd, n_values_cap=cap)

    # Read back with read_ftmb's reader and assert exact string round-trip.
    import read_ftmb as R
    with open(path, "rb") as f:
        ver, n_records, c_d, e_d, s_d, h_d, n_groups, v_d = R.read_header(f)
        assert ver == 6, ver
        sib, records = R.read_v3_group(f, c_d, e_d, s_d, h_d, v_d, ver)
    got = [rec[-1] for rec in records]  # values is the last tuple element for v6
    expect = [vs[:cap] for vs in value_sets]
    assert got == expect, f"value round-trip mismatch:\n got={got}\n exp={expect}"
    print(f"[smoke-v6] OK version=6 cap={cap}; values round-trip exactly "
          f"(unicode + over-cap truncation): {got}")


V19_ARGS = [
    "--distilled", "output/distillation-v3/sherlock_distilled.csv.gz",
    "--finetype", "./target/release/finetype",
    "--label-remap", "data/label_remap.json",
    "--samples-per-type", "1200",
    "--synthetic-columns", "1200",
    "--ratio-distilled", "0.7",
    "--augmentation-rate", "0.35",
    "--filter-distilled",
    "--decontaminate",
    "--distilled-cap", "600",
    "--hard-negatives", "75",
    "--accounting-negatives", "50",
    "--status-negatives", "25",
    "--format", "v4",
    "--seed", "42",
]


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--potion", default="minishlab/potion-base-8M",
                    help="value encoder (model2vec model id or local dir)")
    ap.add_argument("--two-view", default=None,
                    help="second potion to concatenate (e.g. minishlab/potion-base-32M)")
    ap.add_argument("--output", default="output/multibranch-training/m2v8m-244.ftmb")
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--smoke", metavar="PATH", help="write a tiny v5 + verify, then exit")
    ap.add_argument("--smoke-embed", action="store_true",
                    help="check potion embed dim + empty->zeros, then exit")
    ap.add_argument("--smoke-v6", metavar="PATH",
                    help="write a tiny v6 + round-trip its value strings, then exit")
    ap.add_argument("--store-values", type=int, default=0, metavar="N",
                    help="store up to N raw value strings per record and write FTMB v6 "
                         "(per-value attention, choice 0106). 0 = off (v5).")
    ap.add_argument("--cede-labels", default=None, metavar="PATH",
                    help="path to a ceded-leaf list (labels/ceded_leaves.txt). Records carrying "
                         "these labels are dropped as training targets so the leaf leaves the "
                         "model's output label space (spec 2026-06-27-model-label-space-reshape).")
    args = ap.parse_args()

    global _POTIONS
    _POTIONS = [args.potion] + ([args.two_view] if args.two_view else [])

    if args.smoke:
        smoke_write(args.smoke)
        return
    if args.smoke_embed:
        smoke_embed()
        return
    if args.smoke_v6:
        smoke_v6(args.smoke_v6)
        return

    install_patches()
    if args.store_values > 0:
        P.STORE_VALUES_CAP = args.store_values
    argv = ["prepare_multibranch_data.py"] + V19_ARGS + [
        "--output", args.output, "--workers", str(args.workers)]
    if args.cede_labels:
        argv += ["--cede-labels", args.cede_labels]
    mode = " ++ ".join(_POTIONS)
    out_ver = "v6" if args.store_values > 0 else f"v{P.VERSION_V4}"
    print(f"[build] potion embed swap [{mode}]; EMBED_DIM={P.EMBED_DIM} "
          f"VALID_DIM={P.VALID_DIM} VERSION={out_ver} "
          f"store_values={P.STORE_VALUES_CAP}")
    sys.argv = argv
    P.main()


if __name__ == "__main__":
    main()

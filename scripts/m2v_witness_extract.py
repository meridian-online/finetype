#!/usr/bin/env python3
"""Extract (m2v aggregated embedding, label) pairs from an FTMB for the m2v-witness
pilot (spec 2026-06-27-m2v-witness-specialiser-pilot). Reuses read_ftmb's parser.
Usage: m2v_witness_extract.py <file.ftmb> <out.npz>"""
import sys, numpy as np
sys.path.insert(0, "scripts")
from read_ftmb import read_header, read_v3_group  # noqa: E402

def extract(path):
    embeds, labels = [], []
    with open(path, "rb") as f:
        version, n, cd, ed, sd, hd, ng, vd = read_header(f)
        while True:
            g = read_v3_group(f, cd, ed, sd, hd, vd, version)
            if g is None:
                break
            _, recs = g
            for rec in recs:
                labels.append(rec[0]); embeds.append(rec[3])  # label, embed_feat
    return np.asarray(embeds, dtype=np.float32), np.asarray(labels)

if __name__ == "__main__":
    X, y = extract(sys.argv[1])
    np.savez(sys.argv[2], X=X, y=y)
    print(f"{sys.argv[1]}: X={X.shape}  classes={len(set(y))}")

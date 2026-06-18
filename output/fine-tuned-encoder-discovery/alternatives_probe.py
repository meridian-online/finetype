"""Head-to-head: MiniLM vs candle-runnable / faster alternatives, on OUR objective —
separability of the 244 contested columns (header+values, 5-fold CV) + single-item
latency. Filters by what fits FineType: small, fast, candle-runnable (BERT-family or
static). The HN thread's big retrieval models (Gemma/Qwen3) are excluded — wrong
direction for a CPU-latency-bound, short-input classification task.
"""
import csv, time
import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import cross_val_score

rows = list(csv.DictReader(open("output/fine-tuned-encoder-discovery/probe_data.tsv"), delimiter="\t"))
texts = [r["text"] for r in rows]; y = np.array([r["true_family"] for r in rows])

def cv(X): return cross_val_score(LogisticRegression(max_iter=2000), np.asarray(X), y, cv=5).mean()

def lat(enc, n=40):
    t = time.perf_counter()
    for i in range(n): enc([texts[i % len(texts)]])
    return (time.perf_counter() - t) / n * 1000

results = []
# --- static (model2vec) — ultra-fast, the speed-optimal direction ---
from model2vec import StaticModel
for mid, label, sz in [("minishlab/potion-base-8M", "static potion-8M", "30MB"),
                       ("minishlab/potion-base-32M", "static potion-32M", "120MB")]:
    try:
        m = StaticModel.from_pretrained(mid)
        acc = cv(m.encode(texts)); l = lat(lambda b: m.encode(b))
        results.append((label, sz, acc, l, "static/instant"))
    except Exception as e:
        print(f"skip {label}: {e}")
# --- BERT-family encoders (candle-runnable) ---
from sentence_transformers import SentenceTransformer
for mid, label, sz in [("sentence-transformers/all-MiniLM-L6-v2", "MiniLM-L6 (incumbent)", "90MB"),
                       ("TaylorAI/gte-tiny", "gte-tiny", "46MB"),
                       ("BAAI/bge-small-en-v1.5", "bge-small", "130MB"),
                       ("thenlper/gte-small", "gte-small", "67MB")]:
    try:
        m = SentenceTransformer(mid, device="cpu")
        acc = cv(m.encode(texts, show_progress_bar=False))
        l = lat(lambda b: m.encode(b, show_progress_bar=False))
        results.append((label, sz, acc, l, "BERT/candle-ok"))
    except Exception as e:
        print(f"skip {label}: {e}")

print(f"\n{'model':28} {'size':7} {'separability':>12} {'lat ms (torch cpu)':>18}  {'candle?':<16}")
for label, sz, acc, l, fam in sorted(results, key=lambda x: -x[2]):
    print(f"{label:28} {sz:7} {acc:>12.3f} {l:>18.2f}  {fam}")
print("\n(separability = 5-fold CV on 244 contested cols, header+values; MiniLM header+values was 0.893)")

#!/usr/bin/env python3
"""Unit tests for the amvg (amount-variant generators) spec.

Spec: 2026-04-24-amount-variant-generators (v1.3)
test_prefix: amvg

These tests assert the structural and consistency contracts declared in
each AC's `verification` clause. They are the machine-checked regression
guard for the diagnostic artefacts under
the 2026-04-24-amount-variant-generators spec's `diagnostics/` directory.

The tests read committed artefacts; they do not regenerate them. The
regeneration scripts (`scripts/amvg/ac0N_*.py`) are the producers. This
file is the verifier. Running order:

  cd <repo-root>
  python3 -m unittest scripts.amvg.test_amvg -v

CI hook: `make amvg-test` (if wired) or direct invocation.
"""
from __future__ import annotations
import csv
import re
import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC_DIR = REPO_ROOT / ".orbit/specs/2026-04-24-amount-variant-generators"
DIAG_DIR = SPEC_DIR / "diagnostics"
MADR_0065 = REPO_ROOT / ".orbit/choices/0065-amount-subtype-collapse-mechanism.md"
MADR_0066 = REPO_ROOT / ".orbit/choices/0066-v19-retrain-hard-gate.md"
MADR_0067 = REPO_ROOT / ".orbit/choices/0067-framing-correction-retrain-is-not-the-lever.md"

TARGET_SUBTYPES = [
    "amount_accounting",
    "amount_apostrophe",
    "amount_code_prefix",
    "amount_comma",
    "amount_comma_suffix",
    "amount_crypto",
    "amount_lakh",
    "amount_multisym",
    "amount_neg_trailing",
    "amount_nodecimal",
    "amount_space",
]
N_CLASSES = 240
UNIFORM_FLOOR = 1.0 / N_CLASSES
NET_LIFT_THRESHOLD = 3
REGRESSION_THRESHOLD = -1


def _read_tsv(path: Path) -> tuple[list[str], list[list[str]]]:
    with open(path) as f:
        r = csv.reader(f, delimiter="\t")
        rows = list(r)
    return rows[0], rows[1:]


class AmvgAc01CorpusCounts(unittest.TestCase):
    """ac-01: Corpus-count-per-subtype table + hash pin."""

    def test_amvg_ac01_corpus_counts(self) -> None:
        counts_path = DIAG_DIR / "corpus_counts.tsv"
        hash_path = DIAG_DIR / "v16_corpus_hash.txt"
        self.assertTrue(counts_path.exists(), f"missing {counts_path}")
        self.assertTrue(hash_path.exists(), f"missing {hash_path}")

        header, rows = _read_tsv(counts_path)
        self.assertEqual(len(rows), 12, "expected 12 rows (11 subtypes + plain amount control)")
        # Committed format uses fully-qualified labels (finance.currency.*); accept either
        # suffix or fully-qualified form to match the on-disk artefact.
        expected_fq = {f"finance.currency.{s}" for s in TARGET_SUBTYPES} | {"finance.currency.amount"}
        expected_suffix = set(TARGET_SUBTYPES) | {"amount"}
        got_subtypes = {r[0] for r in rows}
        self.assertIn(got_subtypes, (expected_fq, expected_suffix),
                      f"row subtype set mismatch: {got_subtypes}")

        for row in rows:
            count = int(row[1])
            self.assertGreaterEqual(count, 0, f"negative count: {row}")

        hash_hex = hash_path.read_text().strip()
        self.assertRegex(hash_hex, r"^[0-9a-f]{64}$", "hash must be 64 hex chars")


class AmvgAc02Jaccard(unittest.TestCase):
    """ac-02: Pairwise value-shape Jaccard matrix."""

    def test_amvg_ac02_jaccard(self) -> None:
        path = DIAG_DIR / "jaccard_matrix.tsv"
        self.assertTrue(path.exists(), f"missing {path}")
        header, rows = _read_tsv(path)
        self.assertEqual(len(rows), 12, "expected 12 data rows")
        self.assertEqual(len(header), 13, "expected header: label + 12 subtype cols")

        matrix: dict[str, dict[str, float]] = {}
        labels = header[1:]
        for row in rows:
            label = row[0]
            matrix[label] = {labels[i]: float(row[i + 1]) for i in range(12)}

        # (a) symmetry (6 dp)
        for a in labels:
            for b in labels:
                self.assertAlmostEqual(matrix[a][b], matrix[b][a], places=6,
                                       msg=f"asymmetric: {a}<->{b}")
        # (b) diagonal == 1.0
        for a in labels:
            self.assertAlmostEqual(matrix[a][a], 1.0, places=6, msg=f"diag {a} != 1")
        # (c) all values in [0, 1]
        for a in labels:
            for b in labels:
                self.assertGreaterEqual(matrix[a][b], 0.0)
                self.assertLessEqual(matrix[a][b], 1.0)
        # (d) not all off-diagonal == 1 (guards against degenerate signature)
        any_lt_099 = any(matrix[a][b] < 0.99 for a in labels for b in labels if a != b)
        self.assertTrue(any_lt_099, "all off-diagonals >= 0.99 — degenerate signature")


class AmvgAc03Confusion(unittest.TestCase):
    """ac-03: Confusion matrix on the 11 eval columns (v16 baseline)."""

    def test_amvg_ac03_confusion(self) -> None:
        preds_path = DIAG_DIR / "predictions.tsv"
        conf_path = DIAG_DIR / "confusion_matrix.tsv"
        self.assertTrue(preds_path.exists(), f"missing {preds_path}")
        self.assertTrue(conf_path.exists(), f"missing {conf_path}")

        # predictions.tsv: 12 rows (11 targets + control)
        _, pred_rows = _read_tsv(preds_path)
        self.assertEqual(len(pred_rows), 12, "predictions.tsv should have 12 data rows")

        # confusion_matrix.tsv: structural
        _, conf_rows = _read_tsv(conf_path)
        target_rows = [r for r in conf_rows if r[0].startswith("finance.currency.amount_")]
        self.assertEqual(len(target_rows), 11,
                         f"expected 11 target rows in confusion matrix, got {len(target_rows)}")


class AmvgAc04Confidence(unittest.TestCase):
    """ac-04: Per-subtype raw-softmax top-5 confidence distribution."""

    def test_amvg_ac04_confidence(self) -> None:
        path = DIAG_DIR / "confidence_dist.tsv"
        self.assertTrue(path.exists(), f"missing {path}")
        header, rows = _read_tsv(path)
        self.assertEqual(header, ["subtype", "rank", "predicted_label", "probability"])
        self.assertEqual(len(rows), 55, f"expected 55 rows (11×5), got {len(rows)}")

        by_subtype: dict[str, list[tuple[int, str, float]]] = {}
        for r in rows:
            sub, rank, label, prob = r[0], int(r[1]), r[2], float(r[3])
            by_subtype.setdefault(sub, []).append((rank, label, prob))
            # (c) prob strictly in (0, 1)
            self.assertGreater(prob, 0.0)
            self.assertLess(prob, 1.0)

        self.assertEqual(set(by_subtype.keys()), set(TARGET_SUBTYPES))
        for sub, triples in by_subtype.items():
            triples.sort()
            ranks = [t[0] for t in triples]
            probs = [t[2] for t in triples]
            # (b) ranks 1..5
            self.assertEqual(ranks, [1, 2, 3, 4, 5], f"{sub} ranks: {ranks}")
            # (d) top-1 >= uniform floor
            self.assertGreaterEqual(probs[0], UNIFORM_FLOOR - 1e-9)
            # (e) non-increasing
            for i in range(1, 5):
                self.assertLessEqual(probs[i], probs[i - 1] + 1e-6,
                                     f"{sub} rank {i+1} > rank {i}")
            # (f) sum in (0, 1.0]
            s = sum(probs)
            self.assertGreater(s, 0.0)
            self.assertLessEqual(s, 1.0 + 1e-6)


class AmvgAc05MadrContract(unittest.TestCase):
    """ac-05: Mechanism MADR contract."""

    ENUM = {"imbalance", "overlap", "confident_wrong", "flat_confidence",
            "multi_cause", "other"}

    def _parse_frontmatter(self, path: Path) -> dict[str, str]:
        lines = path.read_text().splitlines()
        self.assertEqual(lines[0], "---", "MADR must start with frontmatter")
        fm: dict[str, str] = {}
        for line in lines[1:]:
            if line == "---":
                break
            if ":" in line:
                k, _, v = line.partition(":")
                fm[k.strip()] = v.strip().strip('"')
        return fm

    def test_amvg_ac05_madr_contract(self) -> None:
        self.assertTrue(MADR_0065.exists(), f"missing {MADR_0065}")
        body = MADR_0065.read_text()

        fm = self._parse_frontmatter(MADR_0065)
        self.assertEqual(fm.get("status"), "accepted")
        self.assertEqual(fm.get("date-created"), "2026-04-24")
        self.assertIn(fm.get("primary_mechanism", ""), self.ENUM,
                      f"primary_mechanism not in enum: {fm.get('primary_mechanism')}")

        if fm.get("primary_mechanism") == "other":
            self.assertIn("post_fix_assertion", fm,
                          "primary_mechanism=other requires post_fix_assertion frontmatter field")

        # Ruled Out section with >= 3 bullets
        ruled = re.search(r"##\s+Ruled Out\n(.*?)(?:\n##\s|\Z)", body, re.DOTALL)
        self.assertIsNotNone(ruled, "MADR missing '## Ruled Out' section")
        bullets = re.findall(r"^-\s+\*\*", ruled.group(1), re.MULTILINE)
        self.assertGreaterEqual(len(bullets), 3, "Ruled Out needs >= 3 bullets")

        # All four diagnostic paths cited in Context
        for artefact in ["corpus_counts.tsv", "jaccard_matrix.tsv",
                         "confusion.tsv", "predictions.tsv", "confidence_topk.tsv"]:
            self.assertIn(artefact, body, f"MADR missing citation: {artefact}")


class AmvgAc07MechanismReduced(unittest.TestCase):
    """ac-07: post-fix mechanism reduction via MADR post_fix_assertion."""

    def test_amvg_ac07_mechanism_reduced(self) -> None:
        pre_path = DIAG_DIR / "predictions.tsv"
        post_path = DIAG_DIR / "predictions_post.tsv"
        self.assertTrue(pre_path.exists(), f"missing {pre_path}")
        self.assertTrue(post_path.exists(), f"missing {post_path}")

        def index(path: Path) -> dict[str, str]:
            _, rows = _read_tsv(path)
            return {r[0]: r[2] for r in rows}  # column -> predicted

        pre = index(pre_path)
        post = index(post_path)

        # MADR 0065 post_fix_assertion: >= 3 of the 11 target columns flip
        # from `finance.currency.amount` to the expected variant label.
        flips = 0
        for sub in TARGET_SUBTYPES:
            expected = f"finance.currency.{sub}"
            if pre.get(sub) == "finance.currency.amount" and post.get(sub) == expected:
                flips += 1
        self.assertGreaterEqual(flips, 3,
                                f"MADR 0065 post_fix_assertion failed: only {flips}/11 flips")


class AmvgAc08SmokeRun(unittest.TestCase):
    """ac-08: target-delta + dense full-eval artefacts."""

    def test_amvg_ac08_smoke_run(self) -> None:
        delta_path = DIAG_DIR / "v19_smoke_delta.tsv"
        full_path = DIAG_DIR / "v19_smoke_full_eval.tsv"
        self.assertTrue(delta_path.exists(), f"missing {delta_path}")
        self.assertTrue(full_path.exists(), f"missing {full_path}")

        d_header, d_rows = _read_tsv(delta_path)
        self.assertEqual(d_header, ["subtype", "v16_predicted", "v19_predicted",
                                    "v16_correct", "v19_correct", "delta"])
        self.assertEqual(len(d_rows), 11, f"target delta must have 11 rows, got {len(d_rows)}")

        # Per detour: the v19 smoke retrain was skipped as informationally null;
        # v19_predicted column carries the post-fix v16 prediction. ac-08 v1.3
        # drops the "models/sherlock-v19-smoke-seed-42/" directory requirement.
        f_header, f_rows = _read_tsv(full_path)
        self.assertEqual(f_header, ["eval_column", "v16_correct", "v19_correct", "delta"])
        self.assertGreater(len(f_rows), 300,
                           "dense full-eval must have one row per non-target column (>300)")


class AmvgAc09UnblockGate(unittest.TestCase):
    """ac-09: target-subtype unblock gate — verdict vs numeric consistency."""

    def test_amvg_ac09_unblock_gate(self) -> None:
        verdict_path = DIAG_DIR / "v19_smoke_verdict.txt"
        delta_path = DIAG_DIR / "v19_smoke_delta.tsv"
        self.assertTrue(verdict_path.exists(), f"missing {verdict_path}")
        self.assertTrue(delta_path.exists(), f"missing {delta_path}")

        # (a) verdict file contains exactly `GO` or `NO-GO` — bytes-level
        verdict_bytes = verdict_path.read_bytes()
        self.assertIn(verdict_bytes, {b"GO", b"NO-GO"},
                      f"verdict file must be exactly b'GO' or b'NO-GO'; got {verdict_bytes!r}")
        verdict = verdict_bytes.decode("ascii")

        # Compute net_lift from the TSV
        _, rows = _read_tsv(delta_path)
        net_lift = sum(int(r[5]) for r in rows)

        # (b) GO → net_lift >= 3; (c) NO-GO → net_lift < 3
        if verdict == "GO":
            self.assertGreaterEqual(net_lift, NET_LIFT_THRESHOLD)
        else:
            self.assertLess(net_lift, NET_LIFT_THRESHOLD)


class AmvgAc10RegressionGuard(unittest.TestCase):
    """ac-10: non-target regression guard — verdict vs numeric consistency."""

    def test_amvg_ac10_regression_guard(self) -> None:
        verdict_path = DIAG_DIR / "v19_smoke_regression_verdict.txt"
        full_path = DIAG_DIR / "v19_smoke_full_eval.tsv"
        self.assertTrue(verdict_path.exists(), f"missing {verdict_path}")
        self.assertTrue(full_path.exists(), f"missing {full_path}")

        # (a) verdict file contains exactly `PASS` or `FAIL` — bytes-level
        verdict_bytes = verdict_path.read_bytes()
        self.assertIn(verdict_bytes, {b"PASS", b"FAIL"},
                      f"verdict file must be exactly b'PASS' or b'FAIL'; got {verdict_bytes!r}")
        verdict = verdict_bytes.decode("ascii")

        _, rows = _read_tsv(full_path)
        regression_delta = sum(int(r[3]) for r in rows)

        # (b) PASS → regression_delta >= -1; (c) FAIL → regression_delta < -1
        if verdict == "PASS":
            self.assertGreaterEqual(regression_delta, REGRESSION_THRESHOLD)
        else:
            self.assertLess(regression_delta, REGRESSION_THRESHOLD)


if __name__ == "__main__":
    unittest.main(verbosity=2)

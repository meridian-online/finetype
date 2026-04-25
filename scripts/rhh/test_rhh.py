"""Python test harness for rhh diagnostic scripts.

Covers the verification tests named explicitly in the spec:

  * rhh_ac05_boundary — 5 hits / 4 correct (0.800) → model-covered;
                        5 hits / 3 correct (0.600) → model-gap.
  * rhh_ac05_arithmetic — every classified family's no_hint_label_accuracy
                          matches the ratio correct / columns_hit from the
                          counterfactual TSV.
  * rhh_ac09_reproducibility — regenerate_all.sh is present + executable;
                               committed rhh_fingerprints.sha256 matches
                               the current TSV fingerprints on disk. Byte-
                               identical is the gate; drift triggers a clear
                               test failure listing the divergent artefact.

Run: python3 -m pytest scripts/rhh/test_rhh.py -q
  or: python3 scripts/rhh/test_rhh.py  (built-in unittest runner)
"""

from __future__ import annotations

import csv
import hashlib
import importlib.util
import os
import unittest
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]


def _load_ac05() -> Any:
    """Load ac05_classify by path — siblings in scripts/rhh/ have no package
    parent, so importlib is the clean way to bring the module in."""
    spec = importlib.util.spec_from_file_location(
        "ac05_classify", Path(__file__).parent / "ac05_classify.py"
    )
    assert spec is not None and spec.loader is not None
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


class TestAc05Boundary(unittest.TestCase):
    """rhh_ac05_boundary — named verification test in spec ac-05."""

    def test_rhh_ac05_boundary_exact_threshold_is_model_covered(self) -> None:
        klass, acc = _load_ac05().classify(columns_hit=5, correct=4)
        self.assertEqual(klass, "model-covered")
        self.assertAlmostEqual(acc, 0.800, places=3)

    def test_rhh_ac05_boundary_below_threshold_is_model_gap(self) -> None:
        klass, acc = _load_ac05().classify(columns_hit=5, correct=3)
        self.assertEqual(klass, "model-gap")
        self.assertAlmostEqual(acc, 0.600, places=3)

    def test_rhh_ac05_boundary_zero_hits_is_no_hit(self) -> None:
        klass, acc = _load_ac05().classify(columns_hit=0, correct=0)
        self.assertEqual(klass, "no-hit")
        self.assertEqual(acc, 0.0)


class TestAc05Arithmetic(unittest.TestCase):
    """rhh_ac05_arithmetic — named verification test in spec ac-05.

    Every classified family in rhh_classification.tsv must have
    no_hint_label_accuracy == correct / columns_hit (3dp), derivable from
    rhh_counterfactual.tsv.
    """

    def _load(self, path: Path) -> list[dict[str, str]]:
        with path.open(newline="", encoding="utf-8") as fh:
            lines = [ln for ln in fh if not ln.startswith("#")]
        return list(csv.DictReader(lines, delimiter="\t"))

    def test_rhh_ac05_arithmetic_matches_counterfactual(self) -> None:
        classification_path = (
            REPO_ROOT / "diagnostics" / "rhh_classification.tsv"
        )
        counterfactual_path = (
            REPO_ROOT / "diagnostics" / "rhh_counterfactual.tsv"
        )
        if not classification_path.exists():
            self.skipTest("rhh_classification.tsv not generated")
        if not counterfactual_path.exists():
            self.skipTest("rhh_counterfactual.tsv not generated")

        classification_rows = self._load(classification_path)
        cf_rows = self._load(counterfactual_path)

        # Recompute hits/correct from counterfactual and compare.
        per_fam: dict[str, tuple[int, int]] = {}
        for r in cf_rows:
            fam = r["family_id"]
            changed = r["label_changed"] == "1"
            if not changed:
                continue
            hits, correct = per_fam.get(fam, (0, 0))
            hits += 1
            if r["disabled_correct"] == "1":
                correct += 1
            per_fam[fam] = (hits, correct)

        for row in classification_rows:
            fam = row["family_id"]
            hits_reported = int(row["columns_hit"])
            correct_reported = int(row["no_hint_label_correct"])
            acc_reported = float(row["no_hint_label_accuracy"])
            hits, correct = per_fam.get(fam, (0, 0))
            self.assertEqual(hits_reported, hits, f"{fam}: columns_hit")
            self.assertEqual(correct_reported, correct, f"{fam}: correct")
            expected_acc = correct / hits if hits else 0.0
            self.assertAlmostEqual(
                acc_reported, round(expected_acc, 3), places=3, msg=f"{fam}: acc"
            )


class TestAc09Reproducibility(unittest.TestCase):
    """rhh_ac09_reproducibility — named verification test in spec ac-09.

    The byte-identical gate: current TSV fingerprints must match the
    committed rhh_fingerprints.sha256 manifest. Drift flags a divergent
    artefact by name.
    """

    def test_rhh_ac09_regenerate_script_present_and_executable(self) -> None:
        script = REPO_ROOT / "scripts" / "rhh" / "regenerate_all.sh"
        self.assertTrue(script.is_file(), f"{script} missing")
        self.assertTrue(
            os.access(script, os.X_OK), f"{script} is not executable"
        )

    def test_rhh_ac09_fingerprints_match(self) -> None:
        fp_file = REPO_ROOT / "diagnostics" / "rhh_fingerprints.sha256"
        self.assertTrue(fp_file.is_file(), f"{fp_file} missing")
        expected: dict[str, str] = {}
        with fp_file.open() as fh:
            for raw in fh:
                line = raw.strip()
                if not line or line.startswith("#"):
                    continue
                # sha256sum format: "<hex>  <path>"
                hexhash, _, path = line.partition("  ")
                if not hexhash or not path:
                    continue
                expected[path] = hexhash
        self.assertGreater(
            len(expected), 0, "rhh_fingerprints.sha256 has no entries"
        )
        for rel_path, want in expected.items():
            abs_path = REPO_ROOT / rel_path
            self.assertTrue(abs_path.is_file(), f"{rel_path} missing on disk")
            got = hashlib.sha256(abs_path.read_bytes()).hexdigest()
            self.assertEqual(
                got,
                want,
                f"fingerprint drift on {rel_path}\n  expected: {want}\n  got:      {got}",
            )


if __name__ == "__main__":
    # Allow `python3 scripts/rhh/test_rhh.py` to work without pytest.
    import sys

    sys.path.insert(0, str(Path(__file__).parent))
    unittest.main()

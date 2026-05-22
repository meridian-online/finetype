#!/usr/bin/env python3
"""Tests for scripts/build_corroborated_gaps.py — pure-data unit tests
plus an end-to-end integration test using synthetic parquet inputs.

Run with:
    source eval/gittables/.venv/bin/activate
    python3 -m unittest scripts.test_build_corroborated_gaps
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))

from build_corroborated_gaps import (  # noqa: E402
    Row,
    _cascade_flags,
    _ydf_flags,
    char_class,
    gap_id_for,
    split_samples,
    value_pattern,
    value_shape_signature,
)


class CharClassTests(unittest.TestCase):
    def test_uppercase(self):
        for c in ("A", "M", "Z"):
            self.assertEqual(char_class(c), "A")

    def test_lowercase(self):
        for c in ("a", "m", "z"):
            self.assertEqual(char_class(c), "a")

    def test_digit(self):
        for c in ("0", "5", "9"):
            self.assertEqual(char_class(c), "9")

    def test_other(self):
        for c in ("@", ".", " ", "-", "_", "(", "国"):
            self.assertEqual(char_class(c), ".")


class ValuePatternTests(unittest.TestCase):
    def test_email_shape(self):
        self.assertEqual(
            value_pattern("alice@example.com"),
            "aaaaa.aaaaaaa.aaa",
        )

    def test_phone_shape(self):
        # `+44 7700 900123` → ..99.9999.999999
        self.assertEqual(
            value_pattern("+44 7700 900123"),
            ".99.9999.999999",
        )

    def test_empty_string(self):
        self.assertEqual(value_pattern(""), "")


class ValueShapeSignatureTests(unittest.TestCase):
    def test_deterministic(self):
        s = ["alice@example.com", "bob@x.org"]
        self.assertEqual(
            value_shape_signature(s),
            value_shape_signature(s),
        )

    def test_order_independent(self):
        a = ["alice@example.com", "bob@x.org"]
        b = ["bob@x.org", "alice@example.com"]
        self.assertEqual(value_shape_signature(a), value_shape_signature(b))

    def test_dedup_collapses_uniform_shape(self):
        # Three samples sharing one pattern produce same signature
        # as one sample of that pattern.
        uniform = ["abc", "def", "ghi"]
        single = ["abc"]
        self.assertEqual(
            value_shape_signature(uniform),
            value_shape_signature(single),
        )

    def test_distinct_shapes_distinct_signatures(self):
        self.assertNotEqual(
            value_shape_signature(["abc"]),
            value_shape_signature(["123"]),
        )

    def test_empty_samples(self):
        # All-empty inputs are filtered out before hashing; signature
        # is the SHA256 of the empty string (the empty set of patterns
        # serialised as ""). This is the only legal interpretation of
        # "no shapes observed".
        sig = value_shape_signature(["", "", ""])
        expected = hashlib.sha256(b"").hexdigest()
        self.assertEqual(sig, expected)


class GapIdTests(unittest.TestCase):
    def test_deterministic(self):
        pairs = [("/a/b.parquet", "col1"), ("/a/c.parquet", "col2")]
        self.assertEqual(
            gap_id_for("non_trivial_floor", "misclassification", pairs),
            gap_id_for("non_trivial_floor", "misclassification", pairs),
        )

    def test_order_independent(self):
        pairs_a = [("/a/b.parquet", "col1"), ("/a/c.parquet", "col2")]
        pairs_b = [("/a/c.parquet", "col2"), ("/a/b.parquet", "col1")]
        self.assertEqual(
            gap_id_for("non_trivial_floor", "enum_overfit", pairs_a),
            gap_id_for("non_trivial_floor", "enum_overfit", pairs_b),
        )

    def test_criterion_distinguishes(self):
        pairs = [("/a/b.parquet", "col1")]
        self.assertNotEqual(
            gap_id_for("non_trivial_floor", "enum_overfit", pairs),
            gap_id_for("reject_rate_ceil", "enum_overfit", pairs),
        )

    def test_mechanism_distinguishes(self):
        pairs = [("/a/b.parquet", "col1")]
        self.assertNotEqual(
            gap_id_for("non_trivial_floor", "enum_overfit", pairs),
            gap_id_for("non_trivial_floor", "misclassification", pairs),
        )

    def test_dedup_within_pairs(self):
        # Duplicate (file, column) entries collapse — gap_id_for sorts +
        # dedupes the affected-column signature.
        single = [("/a/b.parquet", "col1")]
        duped = [("/a/b.parquet", "col1"), ("/a/b.parquet", "col1")]
        self.assertEqual(
            gap_id_for("non_trivial_floor", "enum_overfit", single),
            gap_id_for("non_trivial_floor", "enum_overfit", duped),
        )


def make_row(**kwargs) -> Row:
    """Row factory with sane defaults; override per test."""
    defaults = dict(
        file_path="/x/a.parquet",
        column_name="col",
        criterion="non_trivial_floor",
        mechanism="misclassification",
        sense_prediction="representation.text.plain_text",
        ydf_prediction=None,
        ydf_confidence=None,
        samples=["abc", "def"],
    )
    defaults.update(kwargs)
    return Row(**defaults)


class YdfFlagsTests(unittest.TestCase):
    def test_no_ydf_does_not_flag(self):
        self.assertFalse(_ydf_flags(make_row()))

    def test_low_confidence_does_not_flag(self):
        r = make_row(
            ydf_prediction="identity.person.email",
            ydf_confidence=0.3,
        )
        self.assertFalse(_ydf_flags(r))

    def test_agreement_does_not_flag(self):
        # YDF predicts the SAME thing as Sense — agreement, not flag.
        r = make_row(
            sense_prediction="identity.person.email",
            ydf_prediction="identity.person.email",
            ydf_confidence=0.9,
        )
        self.assertFalse(_ydf_flags(r))

    def test_high_confidence_disagreement_flags(self):
        r = make_row(
            sense_prediction="representation.text.plain_text",
            ydf_prediction="identity.person.email",
            ydf_confidence=0.9,
        )
        self.assertTrue(_ydf_flags(r))

    def test_threshold_is_inclusive(self):
        # YDF_CONFIDENCE_FLOOR = 0.5; equality flags.
        r = make_row(
            sense_prediction="representation.text.plain_text",
            ydf_prediction="identity.person.email",
            ydf_confidence=0.5,
        )
        self.assertTrue(_ydf_flags(r))


class CascadeFlagsTests(unittest.TestCase):
    def test_prediction_confirmed_does_not_flag(self):
        r = make_row(mechanism="prediction_confirmed")
        self.assertFalse(_cascade_flags(r))

    def test_other_mechanism_flags(self):
        for mech in (
            "misclassification", "enum_overfit",
            "format_diversity_path_a", "unknown_no_fit", "fallthrough",
        ):
            self.assertTrue(_cascade_flags(make_row(mechanism=mech)),
                            f"{mech} should flag")


class SplitSamplesTests(unittest.TestCase):
    def test_pipe_split(self):
        # U+2502 BOX DRAWINGS LIGHT VERTICAL — what corpus_pass uses.
        self.assertEqual(
            split_samples("a│b│c"),
            ["a", "b", "c"],
        )

    def test_drops_empty(self):
        self.assertEqual(split_samples("a││b"), ["a", "b"])

    def test_none(self):
        self.assertEqual(split_samples(None), [])

    def test_empty(self):
        self.assertEqual(split_samples(""), [])


class EndToEndTest(unittest.TestCase):
    """Run the full script against synthetic parquet inputs.

    Builds tiny columns.parquet + mechanism_decomposition.parquet
    fixtures crafted to hit each lens-vote outcome (both-flag,
    ydf-only, cascade-only, neither). Verifies the output parquet
    has the corroborated rows the AND filter expects and the TSV
    audit log has the single-lens rows.
    """

    def _make_fixtures(self, tmp: Path) -> tuple[Path, Path, Path, Path]:
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore

        # 4 columns, one per lens-vote outcome:
        #
        # 1. ("f1.parquet", "c_both") — YDF disagrees high-conf,
        #     cascade emits misclassification → BOTH FLAG (corroborated)
        # 2. ("f1.parquet", "c_ydf_only") — YDF disagrees but cascade
        #     emits prediction_confirmed → YDF only (single-lens)
        # 3. ("f2.parquet", "c_cascade_only") — YDF agrees, cascade
        #     emits enum_overfit → cascade only (single-lens)
        # 4. ("f2.parquet", "c_neither") — YDF agrees + cascade
        #     prediction_confirmed → neither (dropped silently)
        cols_data = [
            {"file_path": "/x/f1.parquet", "column_name": "c_both",
             "sense_prediction": "representation.text.plain_text",
             "ydf_prediction": "identity.person.email",
             "ydf_confidence": 0.9,
             "sample_values_truncated": "alice@example.com│bob@example.com",
             "is_trivial": True},
            {"file_path": "/x/f1.parquet", "column_name": "c_ydf_only",
             "sense_prediction": "representation.text.plain_text",
             "ydf_prediction": "identity.person.email",
             "ydf_confidence": 0.9,
             "sample_values_truncated": "carol@x.org│dave@x.org",
             "is_trivial": True},
            {"file_path": "/x/f2.parquet", "column_name": "c_cascade_only",
             "sense_prediction": "representation.text.plain_text",
             "ydf_prediction": "representation.text.plain_text",
             "ydf_confidence": 0.9,
             "sample_values_truncated": "ON│OFF│ON",
             "is_trivial": True},
            {"file_path": "/x/f2.parquet", "column_name": "c_neither",
             "sense_prediction": "identity.person.email",
             "ydf_prediction": "identity.person.email",
             "ydf_confidence": 0.9,
             "sample_values_truncated": "alice@x.com│bob@y.com",
             "is_trivial": False},
        ]
        cols_schema = pa.schema([
            ("file_path", pa.string()),
            ("column_name", pa.string()),
            ("sense_prediction", pa.string()),
            ("ydf_prediction", pa.string()),
            ("ydf_confidence", pa.float64()),
            ("sample_values_truncated", pa.string()),
            ("is_trivial", pa.bool_()),
        ])
        columns_pq = tmp / "columns.parquet"
        pq.write_table(
            pa.Table.from_pylist(cols_data, schema=cols_schema),
            columns_pq,
        )

        # mechanism_decomposition has the criterion + mechanism for each
        # of the 4 columns. Per ac-08 contract, prediction_confirmed
        # rows are filtered out of mechanism_decomposition.parquet — so
        # the (#2) ydf_only and (#4) neither rows DO NOT appear here.
        # That's the realistic upstream shape: cascade_flags is
        # effectively always true on rows that reach ac-09.
        #
        # To exercise the single-lens path properly in this test, we
        # include ydf_only-style rows by setting their mechanism to
        # prediction_confirmed anyway — _cascade_flags will return False
        # and they'll route to single_lens_signals.tsv via the
        # ydf-flags-but-cascade-doesn't branch. This makes the test
        # asymmetric to production but is the only way to cover the
        # branch with synthetic data.
        mech_data = [
            {"file_path": "/x/f1.parquet", "column_name": "c_both",
             "criterion": "non_trivial_floor",
             "mechanism_token": "misclassification",
             "recommended_action_class": "training_data_addition",
             "contributing_columns_count_or_reject_count": 1},
            {"file_path": "/x/f1.parquet", "column_name": "c_ydf_only",
             "criterion": "non_trivial_floor",
             "mechanism_token": "prediction_confirmed",
             "recommended_action_class": "",
             "contributing_columns_count_or_reject_count": 1},
            {"file_path": "/x/f2.parquet", "column_name": "c_cascade_only",
             "criterion": "non_trivial_floor",
             "mechanism_token": "enum_overfit",
             "recommended_action_class": "validator_widening",
             "contributing_columns_count_or_reject_count": 1},
            # No row for c_neither — represents a column that didn't
            # contribute to any criterion failure (not in ac-08 output).
        ]
        mech_schema = pa.schema([
            ("file_path", pa.string()),
            ("column_name", pa.string()),
            ("criterion", pa.string()),
            ("mechanism_token", pa.string()),
            ("recommended_action_class", pa.string()),
            ("contributing_columns_count_or_reject_count", pa.int64()),
        ])
        mech_pq = tmp / "mechanism_decomposition.parquet"
        pq.write_table(
            pa.Table.from_pylist(mech_data, schema=mech_schema),
            mech_pq,
        )

        out_corroborated = tmp / "corroborated_gaps.parquet"
        out_single = tmp / "single_lens_signals.tsv"
        return columns_pq, mech_pq, out_corroborated, out_single

    def test_end_to_end(self):
        import pyarrow.parquet as pq  # type: ignore

        with TemporaryDirectory(prefix="ac09-") as td_str:
            tmp = Path(td_str)
            cols_pq, mech_pq, out_corr, out_single = self._make_fixtures(tmp)
            script = REPO / "scripts" / "build_corroborated_gaps.py"
            res = subprocess.run(
                [
                    sys.executable, str(script),
                    "--columns-parquet", str(cols_pq),
                    "--mechanism-decomposition", str(mech_pq),
                    "--out-corroborated", str(out_corr),
                    "--out-single-lens", str(out_single),
                ],
                capture_output=True, text=True, check=False,
            )
            self.assertEqual(res.returncode, 0,
                             f"script failed:\n{res.stderr}")
            summary = json.loads(res.stdout)
            self.assertEqual(summary["n_joined_rows"], 3)
            self.assertEqual(summary["n_both_flag"], 1)
            self.assertEqual(summary["n_ydf_only"], 1)
            self.assertEqual(summary["n_cascade_only"], 1)
            self.assertEqual(summary["n_neither_dropped"], 0)
            self.assertEqual(summary["n_corroborated_gaps"], 1)
            self.assertEqual(summary["n_single_lens_rows"], 2)

            # Corroborated parquet shape
            tbl = pq.read_table(out_corr)
            self.assertEqual(tbl.num_rows, 1)
            row = tbl.to_pylist()[0]
            self.assertEqual(row["criterion"], "non_trivial_floor")
            self.assertEqual(row["mechanism"], "misclassification")
            self.assertEqual(row["recommended_action_class"],
                             "training_data_addition")
            self.assertEqual(row["affected_column_count"], 1)
            self.assertEqual(row["rank_within_cell"], 1)
            self.assertEqual(len(row["corroborating_lenses"]), 2)
            lens_names = {x["lens_name"] for x in row["corroborating_lenses"]}
            self.assertEqual(lens_names, {"ydf", "cascade"})
            # Verification query: no DBpedia in the lens list (design
            # revision 2026-05-21).
            self.assertNotIn("dbpedia", lens_names)
            # gap_id is hex SHA256, 64 chars.
            self.assertEqual(len(row["gap_id"]), 64)
            int(row["gap_id"], 16)  # parses as hex

            # Single-lens TSV: 2 rows (1 ydf-only, 1 cascade-only)
            lines = out_single.read_text().strip().splitlines()
            self.assertEqual(len(lines), 3)  # header + 2 rows
            header = lines[0].split("\t")
            self.assertIn("ydf_flagged", header)
            self.assertIn("cascade_flagged", header)


if __name__ == "__main__":
    unittest.main()

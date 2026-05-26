#!/usr/bin/env python3
"""Tests for scripts/apply_ydf_validation_gate.py — pure-data unit
tests covering each validation kind the gate has to handle.

Run with:
    uv run --with pyyaml python3 -m unittest scripts.test_apply_ydf_validation_gate
"""
from __future__ import annotations

import sys

import regex as re  # type: ignore
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))

from apply_ydf_validation_gate import (  # noqa: E402
    PASS_RATE_THRESHOLD,
    ValidationSpec,
    _compile_spec,
    evaluate,
    load_validations,
)


class CompileSpecTests(unittest.TestCase):
    def test_pattern_and_enum_jointly_applied(self):
        # When both pattern and enum are present, the spec carries
        # both and `passes()` requires both to hold. `kind` records
        # the most-constraining attached check (enum here).
        body = {
            "validation": {
                "type": "string",
                "pattern": "^[A-Z]{2}$",
                "enum": ["US", "GB"],
            },
        }
        spec = _compile_spec("test", body)
        self.assertEqual(spec.kind, "enum")
        self.assertIsNotNone(spec.pattern_re)
        self.assertIsNotNone(spec.enum_set)
        # Joint semantics: pattern-matching but not in enum → fails.
        self.assertTrue(spec.passes("US"))
        self.assertFalse(spec.passes("FR"))  # passes pattern, not in enum
        self.assertFalse(spec.passes("usa"))  # passes enum-ish, not pattern

    def test_pattern_only(self):
        body = {"validation": {"pattern": "^[A-Z]{2}$"}}
        spec = _compile_spec("test", body)
        self.assertEqual(spec.kind, "pattern")
        self.assertIsNotNone(spec.pattern_re)
        self.assertIsNone(spec.enum_set)

    def test_enum_only(self):
        body = {"validation": {"type": "string", "enum": ["yes", "no"]}}
        spec = _compile_spec("test", body)
        self.assertEqual(spec.kind, "enum")
        assert spec.enum_set is not None
        self.assertIn("yes", spec.enum_set)
        self.assertIsNone(spec.pattern_re)

    def test_country_code_enum_rejects_state_codes(self):
        # The contamination workaround is gone — the yaml's enum is
        # canonical 249 ISO 3166-1 alpha-2 codes. With joint pattern+
        # enum application, US-state-shaped values that aren't in the
        # ISO list are rejected.
        body = {
            "validation": {
                "pattern": "^[A-Z]{2}$",
                "enum": ["US", "GB", "AL", "CA"],  # AL=Albania, CA=Canada
            },
        }
        spec = _compile_spec("geography.location.country_code", body)
        self.assertEqual(spec.kind, "enum")
        # ISO codes pass:
        self.assertTrue(spec.passes("US"))
        self.assertTrue(spec.passes("AL"))  # Albania
        # State codes not in ISO list fail enum, even though they
        # match the alpha-2 pattern:
        self.assertFalse(spec.passes("UT"))
        self.assertFalse(spec.passes("NY"))

    def test_locale_union(self):
        body = {
            "validation_by_locale": {
                "US": {"pattern": "^\\d{5}(-\\d{4})?$"},
                "GB": {"pattern": "^[A-Z]{1,2}\\d[A-Z\\d]? ?\\d[A-Z]{2}$"},
            },
        }
        spec = _compile_spec("geography.address.postal_code", body)
        self.assertEqual(spec.kind, "locale")
        self.assertEqual(len(spec.locale_patterns), 2)

    def test_range_fallback(self):
        body = {"validation": {"minimum": 1900, "maximum": 2100}}
        spec = _compile_spec("datetime.year", body)
        self.assertEqual(spec.kind, "range")
        self.assertEqual(spec.minimum, 1900.0)
        self.assertEqual(spec.maximum, 2100.0)

    def test_length_only_yields_none(self):
        # Length-only validation is intentionally a no-op for the gate.
        body = {"validation": {"minLength": 2, "maxLength": 200}}
        spec = _compile_spec("text.label", body)
        self.assertEqual(spec.kind, "none")

    def test_no_validation_yields_none(self):
        spec = _compile_spec("test", {"title": "x"})
        self.assertEqual(spec.kind, "none")


class PassesTests(unittest.TestCase):
    def test_pattern_passes(self):
        spec = ValidationSpec(
            label="iso6346", kind="pattern",
            pattern_re=re.compile(r"^[A-Z]{3}[UJZ]\d{7}$"))
        self.assertTrue(spec.passes("ABCU1234567"))
        self.assertFalse(spec.passes("msg48752425"))
        self.assertFalse(spec.passes("123"))

    def test_pattern_uses_fullmatch_not_search(self):
        # Critical: a partial match in the middle of the string must NOT
        # count. We don't want "msg48752425XYZJ1234567" promoted just
        # because a substring matches.
        spec = ValidationSpec(
            label="iso6346", kind="pattern",
            pattern_re=re.compile(r"[A-Z]{3}[UJZ]\d{7}"))  # no anchors
        self.assertFalse(spec.passes("prefix_ABCU1234567_suffix"))

    def test_enum_membership(self):
        spec = ValidationSpec(
            label="boolean", kind="enum",
            enum_set=frozenset(["yes", "no", "true", "false"]))
        self.assertTrue(spec.passes("yes"))
        self.assertFalse(spec.passes("maybe"))

    def test_locale_union(self):
        spec = ValidationSpec(
            label="postal", kind="locale",
            locale_patterns=[
                re.compile(r"^\d{5}$"),
                re.compile(r"^[A-Z]\d[A-Z] \d[A-Z]\d$"),
            ])
        self.assertTrue(spec.passes("90210"))
        self.assertTrue(spec.passes("M5V 3A8"))
        self.assertFalse(spec.passes("XYZ"))

    def test_range(self):
        spec = ValidationSpec(
            label="year", kind="range", minimum=1900.0, maximum=2100.0)
        self.assertTrue(spec.passes("2024"))
        self.assertFalse(spec.passes("1800"))
        self.assertFalse(spec.passes("notanumber"))


class EvaluateTests(unittest.TestCase):
    def test_no_spec_passes_through(self):
        self.assertEqual(evaluate(None, ["any", "values"]), 1.0)

    def test_kind_none_passes_through(self):
        spec = ValidationSpec(label="x", kind="none")
        self.assertEqual(evaluate(spec, ["any", "values"]), 1.0)

    def test_no_values_passes_through(self):
        # No evidence → no gating. Keep the prediction.
        spec = ValidationSpec(
            label="iso6346", kind="pattern",
            pattern_re=re.compile(r"^[A-Z]{3}[UJZ]\d{7}$"))
        self.assertEqual(evaluate(spec, []), 1.0)
        self.assertEqual(evaluate(spec, ["", "   ", ""]), 1.0)

    def test_majority_pass(self):
        spec = ValidationSpec(
            label="iso6346", kind="pattern",
            pattern_re=re.compile(r"^[A-Z]{3}[UJZ]\d{7}$"))
        values = ["ABCU1234567", "ABCU1234568", "ABCU1234569",
                  "ABCU1234570", "ABCU1234571", "notiso6346"]
        rate = evaluate(spec, values)
        self.assertAlmostEqual(rate, 5 / 6, places=3)
        # 0.833 > 0.5 threshold → would be kept
        self.assertGreater(rate, PASS_RATE_THRESHOLD)

    def test_majority_fail(self):
        # The headline failure mode: YDF predicts iso6346 on msg_id values.
        spec = ValidationSpec(
            label="iso6346", kind="pattern",
            pattern_re=re.compile(r"^[A-Z]{3}[UJZ]\d{7}$"))
        values = [f"msg{n:010d}" for n in range(10)]
        rate = evaluate(spec, values)
        self.assertEqual(rate, 0.0)
        self.assertLess(rate, PASS_RATE_THRESHOLD)


class LoadValidationsTests(unittest.TestCase):
    """Smoke test against the real taxonomy yamls — no fixtures.

    Validates that the canonical types we care about for the gate land
    in the expected kind. Catches taxonomy drift that would silently
    weaken the gate.
    """
    def setUp(self):
        self.specs = load_validations()

    def test_taxonomy_loads(self):
        self.assertGreater(len(self.specs), 200)

    def test_iso6346_is_pattern(self):
        self.assertEqual(
            self.specs["geography.transportation.iso6346"].kind, "pattern")

    def test_mgrs_is_pattern(self):
        self.assertEqual(
            self.specs["geography.coordinate.mgrs"].kind, "pattern")

    def test_plus_code_is_pattern(self):
        self.assertEqual(
            self.specs["geography.coordinate.plus_code"].kind, "pattern")

    def test_country_code_uses_enum_jointly_with_pattern(self):
        # The enum is canonical 249 ISO 3166-1 alpha-2 codes; the
        # spec carries both pattern and enum and applies them
        # jointly (matches `CompiledValidator` in finetype-core).
        spec = self.specs["geography.location.country_code"]
        self.assertEqual(spec.kind, "enum")
        self.assertIsNotNone(spec.pattern_re)
        self.assertIsNotNone(spec.enum_set)
        # ISO codes pass; state-shaped non-ISO codes fail.
        self.assertTrue(spec.passes("US"))
        self.assertTrue(spec.passes("ZW"))
        self.assertFalse(spec.passes("UT"))  # Utah — not ISO
        self.assertFalse(spec.passes("OK"))  # Oklahoma — not ISO


if __name__ == "__main__":
    unittest.main()

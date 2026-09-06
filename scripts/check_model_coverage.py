#!/usr/bin/env python3
"""Gate the generated taxonomy schema catalogue against the model it is
released beside — the property that can silently rot because the two move
independently.

The catalogue comes from `labels/definitions_*.yaml` in this repository at the
release tag. The model comes from HuggingFace under a name set by the
workflow-level `FINETYPE_CI_MODEL`, resolved and downloaded by
`.github/scripts/download-model.sh`. A release that advances one and not the
other degrades `brightfield_engine::semantic::catalogue_coverage`'s consumer
without failing anything else in this repository — the CLI still runs, the
extension still loads, the catalogue is still well-formed JSON.

WHAT THIS MEASURES
    `catalogue_coverage`, reproduced here: read the shipped model's
    `label_map.json` (a JSON array of label strings), and count how many of
    those labels the catalogue MENTIONS — i.e. some entry's `x-finetype-label`
    equals that string. The covered fraction is `covered / len(label_map)`.

WHY THE THRESHOLD IS TIGHTER THAN THE CONSUMER'S OWN FLOOR
    brightfield's loader refuses a bundle describing under half the model's
    labels (`< 0.5`) — that is the point past which it will not even try. This
    check's default, `MIN_COVERAGE`, is set well above that floor so a release
    reddens here on real drift, long before it would reach that emergency
    floor in a shipped bundle.

USAGE
    scripts/check_model_coverage.py --catalogue <catalogue.json> \\
        --label-map <label_map.json> [--threshold 0.95]
    scripts/check_model_coverage.py --self-test

EXIT CODES
    0  covered fraction >= threshold
    1  covered fraction < threshold (report which labels are missing)
    2  the check could not run: bad usage, missing/unreadable file, invalid
       JSON, wrong shape, or an empty label map (nothing to measure coverage
       against)

Stdlib only.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# Reproduced comment, not re-derived: brightfield's loader floor is 0.5. This
# check's default sits well above it so drift is caught long before a release
# would actually fail that floor in a shipped bundle.
MIN_COVERAGE = 0.95


def coverage(catalogue: object, label_map: object) -> tuple[float, list[str], list[str]]:
    """Return (fraction, missing_labels, problems).

    `problems` is non-empty when the inputs are not shaped as expected
    (not lists, an empty label map, a label_map entry that is not a string) —
    those are usage/data errors (exit 2), not a real coverage measurement.
    """
    problems: list[str] = []
    if not isinstance(catalogue, list):
        problems.append(f"catalogue top-level value is a {type(catalogue).__name__}, not a JSON array")
    if not isinstance(label_map, list):
        problems.append(f"label map top-level value is a {type(label_map).__name__}, not a JSON array")
    if problems:
        return 0.0, [], problems

    non_string = [m for m in label_map if not isinstance(m, str)]
    if non_string:
        problems.append(f"label map contains {len(non_string)} non-string entr(y/ies)")
        return 0.0, [], problems

    if len(label_map) == 0:
        problems.append("label map is empty — nothing to measure coverage against")
        return 0.0, [], problems

    catalogue_labels = {
        entry.get("x-finetype-label")
        for entry in catalogue
        if isinstance(entry, dict) and isinstance(entry.get("x-finetype-label"), str)
    }

    missing = [label for label in label_map if label not in catalogue_labels]
    covered = len(label_map) - len(missing)
    fraction = covered / len(label_map)
    return fraction, missing, []


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST — a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════


def self_test() -> int:
    good_catalogue = [
        {"x-finetype-label": "identity.person.email", "pattern": "x"},
        {"x-finetype-label": "identity.person.name"},
        {"x-finetype-label": "datetime.date.iso_8601", "pattern": "y"},
    ]
    good_label_map = [
        "identity.person.email",
        "identity.person.name",
        "datetime.date.iso_8601",
    ]

    fraction, missing, problems = coverage(good_catalogue, good_label_map)
    if problems or missing or fraction != 1.0:
        print(f"  CONTROL FAILED — full coverage did not measure 1.0: {fraction=} {missing=} {problems=}")
        return 1
    print("  ok   control: a label map fully mentioned by the catalogue measures 1.0")

    failed = 0

    # Case 1 (AC6): a label map carrying labels the catalogue does not mention.
    drifted_label_map = good_label_map + [
        "geography.address.street",
        "finance.identifier.iban",
        "technology.network.mac_address",
    ]
    fraction, missing, problems = coverage(good_catalogue, drifted_label_map)
    expected_fraction = 3 / 6
    if problems:
        print(f"  MISS a label map with labels the catalogue does not mention: reported problems {problems}")
        failed += 1
    elif abs(fraction - expected_fraction) > 1e-9:
        print(f"  MISS a label map with labels the catalogue does not mention: fraction {fraction} != {expected_fraction}")
        failed += 1
    elif set(missing) != {"geography.address.street", "finance.identifier.iban", "technology.network.mac_address"}:
        print(f"  WRONG a label map with labels the catalogue does not mention: missing set was {missing}")
        failed += 1
    elif not (fraction < MIN_COVERAGE):
        print(f"  WRONG a label map with labels the catalogue does not mention: {fraction} is not below MIN_COVERAGE={MIN_COVERAGE}")
        failed += 1
    else:
        print(f"  ok   a label map with labels the catalogue does not mention measures {fraction:.3f} (< {MIN_COVERAGE}, would fail the gate)")

    # Case 2: an emptied catalogue against a non-empty label map — every label missing.
    fraction, missing, problems = coverage([], good_label_map)
    if problems or fraction != 0.0 or set(missing) != set(good_label_map):
        print(f"  MISS an emptied catalogue: {fraction=} {missing=} {problems=}")
        failed += 1
    else:
        print("  ok   an emptied catalogue against a non-empty label map measures 0.0")

    # Case 3: an empty label map is a usage/data error, not a coverage of 1.0 or 0.0.
    fraction, missing, problems = coverage(good_catalogue, [])
    if not problems:
        print(f"  MISS an empty label map should be flagged as a problem, not silently scored: {fraction=}")
        failed += 1
    else:
        print(f"  ok   an empty label map is refused: {problems}")

    # Case 4: catalogue is not a list.
    fraction, missing, problems = coverage({"not": "a list"}, good_label_map)
    if not problems or "catalogue" not in problems[0]:
        print(f"  MISS a non-array catalogue should be flagged: {problems}")
        failed += 1
    else:
        print(f"  ok   a non-array catalogue is refused: {problems}")

    if failed:
        print(f"\nself-test FAILED: {failed} case(s) not detected correctly")
        return 1
    print("\nself-test passed")
    return 0


# ══════════════════════════════════════════════════════════════════════════════


def _load(path: Path) -> object:
    text = path.read_text(encoding="utf-8")
    return json.loads(text)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--catalogue", type=Path, help="path to the generated catalogue JSON")
    parser.add_argument("--label-map", type=Path, help="path to the model's label_map.json")
    parser.add_argument("--threshold", type=float, default=MIN_COVERAGE, help=f"minimum covered fraction (default {MIN_COVERAGE})")
    parser.add_argument(
        "--write-manifest",
        type=Path,
        default=None,
        help="also record the measurement (model, tag, coverage) as JSON at this path — AC5",
    )
    parser.add_argument("--tag", default=None, help="release tag, recorded in --write-manifest's output")
    parser.add_argument("--model", default=None, help="model name, recorded in --write-manifest's output")
    parser.add_argument("--self-test", action="store_true", help="prove the gate detects")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    if args.catalogue is None or args.label_map is None:
        parser.error("--catalogue and --label-map are required unless --self-test is given")

    if not (0.0 <= args.threshold <= 1.0):
        print(f"error: --threshold must be between 0 and 1, got {args.threshold}", file=sys.stderr)
        return 2

    try:
        catalogue = _load(args.catalogue)
    except OSError as exc:
        print(f"error: could not read {args.catalogue}: {exc}", file=sys.stderr)
        return 2
    except json.JSONDecodeError as exc:
        print(f"error: invalid JSON in {args.catalogue}: {exc}", file=sys.stderr)
        return 2

    try:
        label_map = _load(args.label_map)
    except OSError as exc:
        print(f"error: could not read {args.label_map}: {exc}", file=sys.stderr)
        return 2
    except json.JSONDecodeError as exc:
        print(f"error: invalid JSON in {args.label_map}: {exc}", file=sys.stderr)
        return 2

    fraction, missing, problems = coverage(catalogue, label_map)
    if problems:
        print(f"error: cannot measure coverage of {args.label_map} against {args.catalogue}:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 2

    print(
        f"coverage: {fraction:.4f} ({len(label_map) - len(missing)}/{len(label_map)} model labels "
        f"mentioned in {args.catalogue}), threshold {args.threshold}"
    )

    if args.write_manifest is not None:
        manifest = {
            "tag": args.tag,
            "model": args.model,
            "catalogue": str(args.catalogue),
            "label_map_entries": len(label_map),
            "catalogue_entries": len(catalogue) if isinstance(catalogue, list) else None,
            "covered": len(label_map) - len(missing),
            "coverage_fraction": fraction,
            "threshold": args.threshold,
        }
        args.write_manifest.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {args.write_manifest}")

    if fraction < args.threshold:
        print(f"FAIL: coverage {fraction:.4f} is below threshold {args.threshold}", file=sys.stderr)
        print(f"missing ({len(missing)}): {missing}", file=sys.stderr)
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

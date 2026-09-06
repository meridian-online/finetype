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
import subprocess
import sys
import tempfile
from pathlib import Path

# Reproduced comment, not re-derived: brightfield's loader floor is 0.5. This
# check's default sits well above it so drift is caught long before a release
# would actually fail that floor in a shipped bundle.
MIN_COVERAGE = 0.95


class ShapeError(Exception):
    """Parsed JSON exists but is not shaped the way this check expects.

    Raised by the two `_require_*` narrowing helpers below and caught in
    `coverage`, so the one message a caller sees names the exact defect
    (an object where an array was expected, a non-string label) rather than
    surfacing as an unstructured `TypeError` from indexing or iterating
    something `json.load` handed back typed only as `Any`/`object`.
    """


def _require_array(value: object, what: str) -> list[object]:
    """Narrow `value` to `list[object]` or raise `ShapeError` naming `what`."""
    if not isinstance(value, list):
        raise ShapeError(f"{what} top-level value is a {type(value).__name__}, not a JSON array")
    return value


def _require_label_array(value: object, what: str) -> list[str]:
    """Narrow `value` to `list[str]` or raise `ShapeError` naming `what`."""
    items = _require_array(value, what)
    labels: list[str] = []
    for item in items:
        if not isinstance(item, str):
            raise ShapeError(f"{what} contains a non-string entry: {item!r}")
        labels.append(item)
    return labels


def coverage(catalogue: object, label_map: object) -> tuple[float, list[str], list[str]]:
    """Return (fraction, missing_labels, problems).

    `problems` is non-empty when the inputs are not shaped as expected
    (not lists, an empty label map, a label_map entry that is not a string) —
    those are usage/data errors (exit 2), not a real coverage measurement.
    """
    try:
        catalogue_entries = _require_array(catalogue, "catalogue")
        labels = _require_label_array(label_map, "label map")
    except ShapeError as exc:
        return 0.0, [], [str(exc)]

    if len(labels) == 0:
        return 0.0, [], ["label map is empty — nothing to measure coverage against"]

    catalogue_labels = {
        entry.get("x-finetype-label")
        for entry in catalogue_entries
        if isinstance(entry, dict) and isinstance(entry.get("x-finetype-label"), str)
    }

    missing = [label for label in labels if label not in catalogue_labels]
    covered = len(labels) - len(missing)
    fraction = covered / len(labels)
    return fraction, missing, []


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST — a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════


def _run_cli(argv: list[str]) -> tuple[int, str]:
    """Run THIS file as a real subprocess and return (exit code, combined output).

    The in-process cases below prove `coverage` measures the right fraction.
    They say nothing about whether the PROGRAM acts on it: replacing `main`'s
    `if fraction < args.threshold: ... return 1` with `return 0` leaves every
    one of them green while the gate exits 0 over a catalogue mentioning none
    of the model's labels -- a tag that ships an unusable type source past a
    green CI job, which is the defect this file exists to catch. The measured
    fraction and the exit code are two different altitudes and only the second
    is what CI reads, so these cases go through argv, `main` and `sys.exit`.
    """
    proc = subprocess.run(
        [sys.executable, str(Path(__file__).resolve()), *argv],
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode, proc.stdout + proc.stderr


def _exit_code_cases(catalogue: list[object], label_map: list[str], drifted_label_map: list[str]) -> int:
    """Return the number of exit-code cases that did not behave as stated."""
    failed = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        root = Path(tmpdir)

        def write(name: str, text: str) -> str:
            path = root / name
            path.write_text(text, encoding="utf-8")
            return str(path)

        cat = write("catalogue.json", json.dumps(catalogue))
        emptied_cat = write("emptied-catalogue.json", "[]")
        object_cat = write("object-catalogue.json", '{"not": "a list"}')
        not_json = write("not-json.json", "this is not JSON")
        labels = write("label_map.json", json.dumps(label_map))
        drifted = write("drifted_label_map.json", json.dumps(drifted_label_map))
        empty_labels = write("empty_label_map.json", "[]")
        absent = str(root / "absent.json")

        # Exact codes, never "non-zero": exit 1 means coverage was measured and
        # came in under the threshold, exit 2 means it could not be measured.
        # A defect that swaps one for the other refuses the right input for the
        # wrong reason, and "non-zero" cannot tell them apart. The
        # `--threshold 0.4` case is the other direction: it proves the number
        # is READ rather than that failure is the only outcome this can reach.
        cases: list[tuple[str, list[str], int]] = [
            ("a fully covered label map exits 0", ["--catalogue", cat, "--label-map", labels], 0),
            (
                "a label map with labels the catalogue does not mention exits 1 at the default threshold",
                ["--catalogue", cat, "--label-map", drifted],
                1,
            ),
            (
                "the same drift below an explicit --threshold 0.4 exits 0, so the number is read",
                ["--catalogue", cat, "--label-map", drifted, "--threshold", "0.4"],
                0,
            ),
            (
                "an emptied catalogue against a real label map exits 1",
                ["--catalogue", emptied_cat, "--label-map", labels],
                1,
            ),
            (
                "an empty label map exits 2, not 0 and not 1",
                ["--catalogue", cat, "--label-map", empty_labels],
                2,
            ),
            (
                "a catalogue that is an object exits 2, not 1",
                ["--catalogue", object_cat, "--label-map", labels],
                2,
            ),
            (
                "a catalogue that is not JSON exits 2, not 1",
                ["--catalogue", not_json, "--label-map", labels],
                2,
            ),
            (
                "a label-map path that does not exist exits 2, not 1",
                ["--catalogue", cat, "--label-map", absent],
                2,
            ),
            (
                "a --threshold outside 0..1 exits 2, not 0",
                ["--catalogue", cat, "--label-map", labels, "--threshold", "1.5"],
                2,
            ),
            ("neither --catalogue nor --label-map exits 2, not 0", [], 2),
        ]
        for name, argv, expected in cases:
            code, output = _run_cli(argv)
            if code != expected:
                print(f"  MISS {name}: exited {code}")
                last = output.strip().splitlines()
                print(f"      last line of output: {last[-1] if last else '<none>'}")
                failed += 1
            else:
                print(f"  ok   {name}")

        # AC5: the manifest is what a downstream pin reads to learn which model
        # the catalogue was measured against, so its CONTENT is asserted, not
        # merely its existence -- an empty file would satisfy the latter.
        manifest_path = root / "finetype-model.json"
        code, output = _run_cli(
            [
                "--catalogue", cat,
                "--label-map", labels,
                "--write-manifest", str(manifest_path),
                "--tag", "vSELFTEST",
                "--model", "m2v8m-s43",
            ]
        )
        if code != 0:
            print(f"  MISS --write-manifest on a passing measurement exited {code}, expected 0")
            failed += 1
        elif not manifest_path.exists():
            print("  MISS --write-manifest did not write the manifest")
            failed += 1
        else:
            written = json.loads(manifest_path.read_text(encoding="utf-8"))
            expected_manifest = {
                "tag": "vSELFTEST",
                "model": "m2v8m-s43",
                "coverage_fraction": 1.0,
                "covered": len(label_map),
                "label_map_entries": len(label_map),
                "catalogue_entries": len(catalogue),
            }
            wrong = [
                f"{key}={written.get(key)!r} (expected {value!r})"
                for key, value in expected_manifest.items()
                if written.get(key) != value
            ]
            if wrong:
                print(f"  MISS the manifest does not record the measurement: {', '.join(wrong)}")
                failed += 1
            else:
                print("  ok   the manifest records the tag, the model name and the measured fraction")
    return failed


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

    # Case 4: catalogue is not a list — an object (dict), the shape a real
    # `label_map.json`/catalogue mistakenly serialised as `{}` would parse to.
    fraction, missing, problems = coverage({"not": "a list"}, good_label_map)
    if not problems or "catalogue" not in problems[0]:
        print(f"  MISS a non-array catalogue should be flagged: {problems}")
        failed += 1
    else:
        print(f"  ok   a non-array catalogue is refused: {problems}")

    # Case 5: label_map.json parses as JSON but is an object, not an array of
    # strings — the same shape defect as case 4, on the other input. Named
    # explicitly rather than only exercised as an unhandled `TypeError`: a
    # `dict` is not a `TypeError` away from a `list`, it is a `ShapeError`.
    fraction, missing, problems = coverage(good_catalogue, {"identity.person.email": True})
    if not problems or "label map" not in problems[0]:
        print(f"  MISS a non-array label map (an object) should be flagged: {problems}")
        failed += 1
    else:
        print(f"  ok   a non-array label map (an object, not a JSON array) is refused: {problems}")

    # The measurement cases above end here; what follows drives the same
    # inputs through the process boundary CI actually reads.
    failed += _exit_code_cases(good_catalogue, good_label_map, drifted_label_map)

    if failed:
        print(f"\nself-test FAILED: {failed} case(s) not detected correctly")
        return 1
    print("\nself-test passed: measurements checked, control clean, exit codes pinned")
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

    # `coverage` returning no problems is exactly its guarantee that both
    # parsed values are JSON arrays (and every label_map entry a string) — the
    # same two facts `_require_array`/`_require_label_array` just proved
    # inside it. Reasserted here, explicitly, so the rest of this function
    # works with real types rather than the `object` `json.load` hands back:
    # pyright does not carry a callee's internal narrowing across the call.
    assert isinstance(catalogue, list)
    assert isinstance(label_map, list)

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
            "catalogue_entries": len(catalogue),
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

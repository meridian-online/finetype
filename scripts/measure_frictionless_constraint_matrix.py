#!/usr/bin/env python3
"""Measure which constraint keywords `frictionless` accepts beside each field type.

`finetype-core::frictionless_vocabulary::CONSTRAINT_VOCABULARY` is the vendored
profile's per-branch constraint keys minus `REFERENCE_IMPLEMENTATION_NARROWING`,
and that narrowing is measured rather than transcribed. This is the measurement.

It puts a one-field descriptor carrying one keyword through `frictionless.Package`
for every (type, keyword) pair the profile's field `oneOf` mentions, and prints
both tables in the Rust form the module holds them in.

Nothing in CI runs this: the workspace is Rust and `frictionless` is a Python
package, so adding it to CI would mean a Python toolchain on every pull request
for a table that changes when the pin moves. The gate that ships is
`crates/finetype-mcp/tests/conformance.rs`, and
`crates/finetype-core/tests/frictionless_vocabulary.rs` pins the table against
the profile. Re-run this by hand when either pin moves:

    uv run --with frictionless==5.19.0 scripts/measure_frictionless_constraint_matrix.py

Last run 2026-08-28 against frictionless 5.19.0; its output is the two tables in
crates/finetype-core/src/frictionless_vocabulary.rs.
"""

from __future__ import annotations

import json
import pathlib
import sys

PROFILE = (
    pathlib.Path(__file__).resolve().parent.parent
    / "vendor"
    / "frictionless"
    / "datapackage-profile.json"
)

FIELD_ITEMS_POINTER = (
    "properties",
    "resources",
    "items",
    "properties",
    "schema",
    "properties",
    "fields",
    "items",
)

# One value per keyword, of the shape the profile says that keyword takes. The
# value is never compared against data — only the keyword's presence beside the
# type is under test — but it has to type-check against the profile or the
# rejection would be about the value rather than about the pairing.
SAMPLE = {
    "enum": ["a"],
    "maxLength": 10,
    "minLength": 1,
    "pattern": "^a$",
    "required": True,
    "unique": True,
    "exclusiveMaximum": 10,
    "exclusiveMinimum": 1,
    "maximum": 10,
    "minimum": 1,
    "jsonSchema": {"type": "object"},
}


def profile_branches() -> dict[str, list[str]]:
    node = json.loads(PROFILE.read_text())
    for key in FIELD_ITEMS_POINTER:
        node = node[key]
    return {
        branch["properties"]["type"]["enum"][0]: sorted(
            branch["properties"]["constraints"]["properties"]
        )
        for branch in node["oneOf"]
    }


def accepts(package_cls, ftype: str, keyword: str) -> bool:
    descriptor = {
        "name": "p",
        "resources": [
            {
                "name": "r",
                "path": "d.csv",
                "schema": {
                    "fields": [
                        {
                            "name": "f",
                            "type": ftype,
                            "constraints": {keyword: SAMPLE[keyword]},
                        }
                    ]
                },
            }
        ],
    }
    try:
        package_cls(descriptor)
        return True
    except Exception:
        return False


def main() -> int:
    try:
        import frictionless
        from frictionless import Package
    except ImportError:
        print(
            "frictionless is not importable. Run this with:\n"
            "  uv run --with frictionless==5.19.0 "
            "scripts/measure_frictionless_constraint_matrix.py",
            file=sys.stderr,
        )
        return 2

    branches = profile_branches()
    print(f"// measured against frictionless {frictionless.__version__}")
    print("// CONSTRAINT_VOCABULARY")
    narrowing: list[tuple[str, list[str]]] = []
    for ftype, keywords in branches.items():
        kept = [kw for kw in keywords if accepts(Package, ftype, kw)]
        refused = [kw for kw in keywords if kw not in kept]
        if refused:
            narrowing.append((ftype, refused))
        rendered = ", ".join(f'"{kw}"' for kw in kept)
        print(f'    ("{ftype}", &[{rendered}]),')

    print("// REFERENCE_IMPLEMENTATION_NARROWING")
    for ftype, refused in narrowing:
        rendered = ", ".join(f'"{kw}"' for kw in refused)
        print(f'    ("{ftype}", &[{rendered}]),')

    # The taxonomy declares `list`, which the profile has no branch for. Stated
    # here because the two tables above cannot say anything about a type the
    # profile does not mention.
    print("// types the profile does not pin, probed anyway:")
    for ftype in ("list",):
        bare = {
            "name": "p",
            "resources": [
                {
                    "name": "r",
                    "path": "d.csv",
                    "schema": {"fields": [{"name": "f", "type": ftype}]},
                }
            ],
        }
        try:
            Package(bare)
            verdict = "ACCEPTED"
        except Exception as exc:  # noqa: BLE001 - the message is the measurement
            verdict = f"REJECTED — {exc}"
        print(f"//   {ftype}: {verdict}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

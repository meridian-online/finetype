#!/usr/bin/env python3
"""encoder_dtype.py — store a Model2Vec encoder's embedding matrix in half precision.

A static encoder's `model.safetensors` is a lookup table, not a trained graph: at load
time `Model2VecResources::from_bytes` up-casts whatever it finds to F32 before a single
token is embedded. So the stored dtype is packaging, and storing F32 buys nothing at
inference — it just doubles the bytes that have to be downloaded and, for the release
build, compiled into the binary with `include_bytes!`.

This tool rewrites the named tensors from F32 to F16 in place of a copy, and refuses
rather than damages:

  * **overflow is fatal.** F16 tops out at 65504. A value above it would become `inf`,
    which is silent corruption of a lookup table — so a single out-of-range element
    aborts the conversion with the offending index and value.
  * **underflow is reported.** F16's smallest subnormal is ~5.96e-8. Elements that
    round to zero are counted and printed; they are legal (a zeroed row of a lookup
    table is what an unused token already looks like) but they are never silent.
  * **already-F16 is a no-op**, not an error, so re-running the tool on a converted
    artifact cannot double-convert or corrupt it.

Rounding is IEEE 754 round-half-to-even, from CPython's `struct` `'e'` packer. That is
fully specified by the standard and carries no libm dependence, so the same input file
produces the same output bytes on macOS and on Linux CI — which matters, because these
bytes end up inside a released binary.

Usage:
    scripts/encoder_dtype.py inspect  <model.safetensors>
    scripts/encoder_dtype.py to-f16   <model.safetensors> [--out FILE] [--tensor NAME]...
    scripts/encoder_dtype.py self-test

`to-f16` writes in place unless `--out` is given, and always writes to a temporary file
in the destination directory before renaming, so an interrupted run cannot leave a
half-written model where a whole one used to be.

Exit codes:
    0 ok (including "already F16")
    1 conversion refused — a value does not survive the round trip
    2 usage / unreadable input
"""

from __future__ import annotations

import argparse
import json
import os
import struct
import sys
import tempfile
from pathlib import Path

# The largest finite F16, and the smallest positive subnormal. Written as exact
# decimal literals rather than computed, so this file states the boundary it enforces.
F16_MAX = 65504.0
F16_MIN_SUBNORMAL = 5.960464477539063e-08

# Rewriting more than this many elements at a time costs memory without buying speed.
CHUNK = 1 << 16


class Refused(Exception):
    """The conversion would not survive the round trip. Nothing is written."""


def read_safetensors(path: Path) -> tuple[dict, bytes]:
    """Return (header, data-buffer). Offsets in the header index into the buffer."""
    raw = path.read_bytes()
    if len(raw) < 8:
        raise ValueError(f"{path}: too short to be a safetensors file ({len(raw)} bytes)")
    (header_len,) = struct.unpack_from("<Q", raw, 0)
    if 8 + header_len > len(raw):
        raise ValueError(f"{path}: header length {header_len} runs past the end of the file")
    header = json.loads(raw[8 : 8 + header_len])
    return header, raw[8 + header_len :]


def write_safetensors(path: Path, header: dict, data: bytes) -> None:
    """Write header+data atomically, padding the header so the tensor data is 8-aligned."""
    blob = json.dumps(header, separators=(",", ":"), sort_keys=True).encode()
    pad = (-(len(blob)) % 8)
    blob += b" " * pad
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=path.name + ".", suffix=".part")
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(struct.pack("<Q", len(blob)))
            fh.write(blob)
            fh.write(data)
        os.replace(tmp, path)
    except BaseException:
        if os.path.exists(tmp):
            os.unlink(tmp)
        raise


def f32_to_f16(buf: bytes, name: str) -> tuple[bytes, int]:
    """Convert a little-endian F32 buffer to F16. Returns (bytes, underflow count).

    Raises `Refused` on the first element that would become infinite.
    """
    n = len(buf) // 4
    out = bytearray()
    underflowed = 0
    for start in range(0, n, CHUNK):
        count = min(CHUNK, n - start)
        vals = struct.unpack_from(f"<{count}f", buf, start * 4)
        for i, v in enumerate(vals):
            av = v if v >= 0.0 else -v
            if av > F16_MAX:
                raise Refused(
                    f"{name}[{start + i}] = {v!r} exceeds the F16 maximum {F16_MAX}; "
                    f"storing it as F16 would make it infinite"
                )
            if v != 0.0 and av < F16_MIN_SUBNORMAL:
                underflowed += 1
        out += struct.pack(f"<{count}e", *vals)
    return bytes(out), underflowed


def cmd_inspect(args: argparse.Namespace) -> int:
    header, _ = read_safetensors(args.path)
    total = args.path.stat().st_size
    print(f"{args.path}  {total} bytes")
    for name, info in sorted(header.items()):
        if name == "__metadata__":
            print(f"  __metadata__  {json.dumps(info, sort_keys=True)}")
            continue
        start, end = info["data_offsets"]
        print(f"  {name}  {info['dtype']}  {info['shape']}  {end - start} bytes")
    return 0


def cmd_to_f16(args: argparse.Namespace) -> int:
    header, data = read_safetensors(args.path)
    targets = args.tensor or ["embeddings"]

    missing = [t for t in targets if t not in header]
    if missing:
        print(
            f"FAIL: {args.path} has no tensor named {', '.join(missing)} "
            f"(found: {', '.join(sorted(k for k in header if k != '__metadata__'))})",
            file=sys.stderr,
        )
        return 2

    # Rebuild the whole data buffer in header order so the offsets stay contiguous
    # and ascending, which is what every safetensors reader expects.
    ordered = sorted(
        ((n, i) for n, i in header.items() if n != "__metadata__"),
        key=lambda kv: kv[1]["data_offsets"][0],
    )

    converted, skipped, underflow_total = [], [], 0
    new_header: dict = {}
    if "__metadata__" in header:
        new_header["__metadata__"] = header["__metadata__"]
    buf = bytearray()

    for name, info in ordered:
        start, end = info["data_offsets"]
        payload = data[start:end]
        dtype = info["dtype"]
        if name in targets:
            if dtype == "F16":
                skipped.append(f"{name} (already F16)")
            elif dtype != "F32":
                print(
                    f"FAIL: {name} is {dtype}; this tool only converts F32 -> F16",
                    file=sys.stderr,
                )
                return 2
            else:
                try:
                    payload, under = f32_to_f16(payload, name)
                except Refused as e:
                    print(f"FAIL: {e}", file=sys.stderr)
                    print("      nothing was written", file=sys.stderr)
                    return 1
                underflow_total += under
                dtype = "F16"
                converted.append(name)
        offset = len(buf)
        buf += payload
        new_header[name] = {
            "dtype": dtype,
            "shape": info["shape"],
            "data_offsets": [offset, len(buf)],
        }

    dest = args.out or args.path
    before = args.path.stat().st_size
    write_safetensors(dest, new_header, bytes(buf))
    after = dest.stat().st_size

    for s in skipped:
        print(f"unchanged: {s}")
    if converted:
        print(f"converted to F16: {', '.join(converted)}")
    if underflow_total:
        print(f"{underflow_total} element(s) rounded to zero (below the F16 subnormal floor)")
    print(f"{args.path} {before} bytes -> {dest} {after} bytes")
    return 0


# ── self-test ───────────────────────────────────────────────────────────────────
# Runs on stdlib alone so CI needs no scientific stack. Each case names the failure
# it prevents; a case that cannot fail is not a case.


def _build(tensors: dict[str, tuple[str, list[int], bytes]]) -> bytes:
    header, buf = {}, bytearray()
    for name, (dtype, shape, payload) in tensors.items():
        offset = len(buf)
        buf += payload
        header[name] = {"dtype": dtype, "shape": shape, "data_offsets": [offset, len(buf)]}
    blob = json.dumps(header, separators=(",", ":"), sort_keys=True).encode()
    blob += b" " * (-(len(blob)) % 8)
    return struct.pack("<Q", len(blob)) + blob + bytes(buf)


def cmd_self_test(_args: argparse.Namespace) -> int:
    failures: list[str] = []

    def check(label: str, ok: bool, detail: str = "") -> None:
        print(f"{'ok  ' if ok else 'FAIL'}  {label}{'  ' + detail if detail else ''}")
        if not ok:
            failures.append(label)

    with tempfile.TemporaryDirectory() as td:
        root = Path(td)

        # 1. round trip: values exactly representable in F16 come back bit-identical,
        #    so a conversion that silently mangled the table would show up here.
        exact = [0.0, 1.0, -1.0, 0.5, -0.5, 2.0, 0.25, 1024.0, -2048.0, 65504.0]
        p = root / "exact.safetensors"
        p.write_bytes(
            _build({"embeddings": ("F32", [2, 5], struct.pack(f"<{len(exact)}f", *exact))})
        )
        rc = cmd_to_f16(argparse.Namespace(path=p, out=None, tensor=["embeddings"]))
        header, data = read_safetensors(p)
        got = list(struct.unpack(f"<{len(exact)}e", data))
        check("exactly-representable values survive unchanged", rc == 0 and got == exact, str(got))
        check("dtype is rewritten in the header", header["embeddings"]["dtype"] == "F16")
        check("shape is preserved", header["embeddings"]["shape"] == [2, 5])
        check("file halves in size", len(data) == len(exact) * 2)

        # 2. re-running must be a no-op, not a second conversion.
        first = p.read_bytes()
        rc = cmd_to_f16(argparse.Namespace(path=p, out=None, tensor=["embeddings"]))
        check("re-running on an F16 file changes nothing", rc == 0 and p.read_bytes() == first)

        # 3. round-half-to-even, the rule that makes the output platform-independent.
        #    2049 sits exactly between the F16 neighbours 2048 and 2050 -> ties to 2048.
        p2 = root / "tie.safetensors"
        p2.write_bytes(_build({"embeddings": ("F32", [1], struct.pack("<f", 2049.0))}))
        cmd_to_f16(argparse.Namespace(path=p2, out=None, tensor=["embeddings"]))
        _, d2 = read_safetensors(p2)
        (tie,) = struct.unpack("<e", d2)
        check("a tie rounds to even, not away from zero", tie == 2048.0, str(tie))

        # 4. overflow is refused and the original is left intact. This is the case
        #    that separates "converted" from "corrupted": without it, an out-of-range
        #    weight becomes inf and every embedding through that row becomes NaN.
        p3 = root / "big.safetensors"
        original = _build({"embeddings": ("F32", [2], struct.pack("<2f", 1.0, 70000.0))})
        p3.write_bytes(original)
        rc = cmd_to_f16(argparse.Namespace(path=p3, out=None, tensor=["embeddings"]))
        check("a value above 65504 is refused", rc == 1, f"rc={rc}")
        check("a refused conversion leaves the file byte-identical", p3.read_bytes() == original)

        # 5. other tensors in the same file are carried through untouched, with their
        #    offsets rebuilt — an off-by-one here would silently shift every weight.
        side = struct.pack("<3f", 7.0, 8.0, 9.0)
        p4 = root / "multi.safetensors"
        p4.write_bytes(
            _build(
                {
                    "embeddings": ("F32", [2], struct.pack("<2f", 1.5, -1.5)),
                    "other": ("F32", [3], side),
                }
            )
        )
        cmd_to_f16(argparse.Namespace(path=p4, out=None, tensor=["embeddings"]))
        h4, d4 = read_safetensors(p4)
        os_, oe = h4["other"]["data_offsets"]
        es, ee = h4["embeddings"]["data_offsets"]
        check("a sibling tensor keeps its dtype", h4["other"]["dtype"] == "F32")
        check("a sibling tensor keeps its bytes", d4[os_:oe] == side)
        check("the converted tensor is half the width", ee - es == 4)
        check("offsets stay contiguous", sorted([(es, ee), (os_, oe)])[0][0] == 0)
        check("the buffer is exactly the tensors", len(d4) == (ee - es) + (oe - os_))

        # 6. a missing tensor name is a usage error, not a silent success.
        p5 = root / "named.safetensors"
        p5.write_bytes(_build({"weights": ("F32", [1], struct.pack("<f", 1.0))}))
        rc = cmd_to_f16(argparse.Namespace(path=p5, out=None, tensor=["embeddings"]))
        check("a missing tensor name is rejected", rc == 2, f"rc={rc}")

    print()
    if failures:
        print(f"{len(failures)} case(s) failed: {', '.join(failures)}", file=sys.stderr)
        return 1
    print("all cases passed")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)

    p_i = sub.add_parser("inspect", help="print each tensor's dtype, shape and size")
    p_i.add_argument("path", type=Path)
    p_i.set_defaults(func=cmd_inspect)

    p_c = sub.add_parser("to-f16", help="rewrite F32 tensors as F16")
    p_c.add_argument("path", type=Path)
    p_c.add_argument("--out", type=Path, default=None, help="write here instead of in place")
    p_c.add_argument(
        "--tensor",
        action="append",
        default=None,
        help="tensor to convert (repeatable; default: embeddings)",
    )
    p_c.set_defaults(func=cmd_to_f16)

    p_t = sub.add_parser("self-test", help="run the built-in cases (no arguments, no deps)")
    p_t.set_defaults(func=cmd_self_test)

    args = ap.parse_args(argv)
    try:
        return args.func(args)
    except (OSError, ValueError, json.JSONDecodeError) as e:
        print(f"FAIL: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())

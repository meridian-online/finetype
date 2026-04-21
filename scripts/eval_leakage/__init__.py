"""Shared leakage-prevention primitives for FineType's train↔eval firewall.

This module is the SINGLE source of truth for the (header, value) normalisation
spec used by both:

  - scripts/compute_row_hashes.py (ac-06) — writes eval/row_hashes.tsv
  - scripts/prepare_multibranch_data.py (ac-07) — filters training rows

Keeping both sides in one module prevents the classic drift bug where the
hash-generator and the filter silently disagree on what counts as "the same
row". See orbit/decisions/0056-train-eval-leakage-prevention.md for the
full rationale + enumerated blind spots.

Normalisation spec (MUST match MADR 0056):

    normalised_header = lower(strip(header))
    normalised_value  = strip(value) with Unicode NFC and CRLF → LF

Hash spec:

    row_hash = sha256(normalised_header + NUL + normalised_value).hex()

where NUL is a single 0x00 byte that cannot appear in normalised text,
preventing header/value boundary confusion.

Known blind spots (see MADR 0056 §Consequences):
  - Format-drift (timestamps, number precision) — not caught
  - Header synonyms (email vs e-mail) — not caught
  - Whitespace beyond NFC+newline collapse (NBSP, tabs, internal) — not caught
"""

from __future__ import annotations

import hashlib
import unicodedata

__all__ = [
    "normalise_header",
    "normalise_value",
    "row_hash",
    "NORMALISER_VERSION",
]


# Bump this when the normalisation spec changes. Training runs against a
# newer version must regenerate eval/row_hashes.tsv as a pre-flight step.
NORMALISER_VERSION = "1.0.0"


def normalise_header(header: str) -> str:
    """Normalise a column header for hashing.

    Spec: lower(strip(header)).
    """
    return (header or "").strip().lower()


def normalise_value(value: str | None) -> str:
    """Normalise a column value for hashing.

    Spec: strip(value) with Unicode NFC and CRLF → LF.
    """
    if value is None:
        return ""
    s = str(value)
    # CRLF → LF first so trailing CR doesn't get caught by strip()
    s = s.replace("\r\n", "\n").replace("\r", "\n")
    s = unicodedata.normalize("NFC", s)
    return s.strip()


def row_hash(header: str, value: str) -> str:
    """Compute the row hash for a (header, value) pair.

    Returns the hex digest of SHA256 over:
        normalise_header(header) || '\\x00' || normalise_value(value)
    """
    nh = normalise_header(header)
    nv = normalise_value(value)
    payload = nh.encode("utf-8") + b"\x00" + nv.encode("utf-8")
    return hashlib.sha256(payload).hexdigest()

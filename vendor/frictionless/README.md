# Vendored Frictionless profile

`datapackage-profile.json` is the **canonical, self-contained Frictionless Data
Package v2.0 profile** for the Meridian family (choice 0105). FineType owns the
label→Frictionless type map, so it also hosts the one vendored copy of the profile
that `profile -o datapackage` descriptors are validated against — and that dovetail
and arcform reuse rather than each carrying a divergent copy.

## Provenance

- **Source:** `frictionlessdata/datapackage`, `public/profiles/2.0/datapackage.json`
  (the pre-built, published profile served at
  `https://datapackage.org/profiles/2.0/datapackage.json`).
- **Upstream commit:** `6a201af` (2026-05-05).
- **Vendored:** 2026-06-24. Copied **verbatim** — no edits.
- **Upstream licence:** public domain (Unlicense); attribution here is courtesy.

## Why verbatim (not a hand-derived subset)

The v1.0 profiles upstream ship as `$ref` stubs pointing at a shared dictionary,
so they cannot drive a validator standalone — a consumer had to hand-derive a
self-contained subset (this is what dovetail's v1.0 `datapackage-profile.json`
did). The **v2.0** `public/` build is already fully self-contained (0 external
refs, 0 internal refs, draft-07, 3160 lines), so the real artifact is vendored
directly. No drift, no subset to maintain.

## Conformance notes (verified 2026-06-24)

- Sets `additionalProperties: false` **nowhere**, so namespaced custom properties
  (`x-finetype-*`, `x-dovetail*`) validate cleanly — extensions are not rejected.
- Does **not** enum-constrain a field's `type` (it is a freeform string), so every
  v2 type FineType emits — including `list`, `geopoint`, `year`, `yearmonth`,
  `duration` — passes. The type *vocabulary* is enforced by FineType's own
  `Frictionless::validate` (a stricter gate than the profile), not by this schema.

## Consumers

- FineType: spec `2026-06-24-frictionless-datapackage-profile-output` ac-04 (the
  conformance test validates `profile -o datapackage` output against this file).
- dovetail: Phase 2 of memo `2026-06-24-finetype-owns-frictionless-type-map` —
  move both profile pins to `…/profiles/2.0/…`, re-vendor against this exact file,
  re-run `cargo test -p dovetail-core`.

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

## Conformance notes (verified 2026-06-24; the `type` bullet corrected 2026-08-28)

- Sets `additionalProperties: false` **nowhere**, so namespaced custom properties
  (`x-finetype-*`, `x-dovetail*`) validate cleanly — extensions are not rejected.
- **Enum-pins a field's `type` across fifteen `oneOf` branches**, and gives each
  branch its own `constraints` vocabulary. `properties.resources.items.properties
  .schema.properties.fields.items.oneOf` is fifteen alternatives — `string`,
  `number`, `integer`, `date`, `time`, `datetime`, `year`, `yearmonth`,
  `boolean`, `object`, `geopoint`, `geojson`, `array`, `duration`, `any` — each
  pinning `type` to one value with `enum`. Two consequences, both of which cost
  a published descriptor:
  - **A type outside the fifteen fails every branch, and the whole descriptor
    with it.** The taxonomy declares one: `container.array.comma_separated` maps
    to `list`, a Data Package v2 *specification* type this published profile
    build does not carry. `frictionless==5.19.0` refuses it too —
    `field type "list" is not supported`. `finetype-core`'s
    `FRICTIONLESS_TYPES` is the taxonomy's sixteen-entry declaration vocabulary
    and permits it, so `Frictionless::validate` is **weaker** here than the
    profile, not stricter. Pinned by
    `crates/finetype-mcp/tests/conformance.rs`.
  - **Each branch's `constraints` has its own properties**, disjoint across type
    families: `pattern`/`minLength`/`maxLength` belong to `string`,
    `minimum`/`maximum` to the numeric and temporal types. The profile itself
    does not reject a stray one, because it sets `additionalProperties` nowhere
    (see the bullet above) — but `frictionless==5.19.0` does, from its own
    per-type `supported_constraints`, with
    `constraint "pattern" is not supported by type "integer"`. Measured
    2026-08-28: that refused three of the four descriptors published from this
    engine. `finetype-core::frictionless_vocabulary` owns the type→vocabulary
    answer for both emitters; `crates/finetype-core/tests/frictionless_vocabulary.rs`
    reads these branches back out of this file and reddens if the table drifts.
- **The reference implementation is not identical to this profile — it lags it.**
  `frictionless==5.19.0` refuses `exclusiveMinimum`/`exclusiveMaximum` on every
  type the branch allows them on, `jsonSchema` on `object` and `array`,
  `minLength`/`maxLength` on `geojson`, and `minimum`/`maximum` on `duration`.
  It also accepts `unique` on `boolean`, which no branch here allows. Clearing
  this profile is therefore necessary and not sufficient; the emitters target
  the intersection. Re-measure with
  `scripts/measure_frictionless_constraint_matrix.py`.
- **`resource.path` MUST be relative** to the descriptor's directory — an absolute
  path (`/…`), `../`, `~`, or a `file:`/URL scheme is **rejected** by 2.0 (it was
  lenient under 1.0). Emit the basename when the descriptor is co-located with its
  data. FineType's emitter does this via `file_name()`; a downstream consumer
  (dovetail) hit this emitting the absolute source path. A closed `enum` constraint
  must also be type-consistent — observed-string enums only belong on `string`
  fields, not `boolean` (the field `oneOf` rejects the mismatch).

## Consumers

- FineType: spec `2026-06-24-frictionless-datapackage-profile-output` ac-04 (the
  conformance test validates `profile -o datapackage` output against this file).
- FineType: `finetype-core::frictionless_vocabulary`, which answers *which
  constraint keywords may sit beside this type* for both Data Package emitters
  in the family — `finetype-mcp::datapackage` and dovetail's
  `dovetail-core::datapackage`. It is a `finetype-core` export precisely so the
  second emitter adopts the answer rather than copying the table.
- dovetail: Phase 2 of memo `2026-06-24-finetype-owns-frictionless-type-map` —
  move both profile pins to `…/profiles/2.0/…`, re-vendor against this exact file,
  re-run `cargo test -p dovetail-core`. `dovetail-core` already depends on
  `finetype-core`, so `constraint_vocabulary` is reachable there today.

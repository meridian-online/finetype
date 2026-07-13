# Gold-audit queue — value-evidence gold defects awaiting a panel round

Candidates where full-column value evidence contradicts the *current* gold label. These are the
external band's gold-audit role (memory `external-band-triage-gold-growth`): panel-proposes /
author-ratifies, **decoupled from any model/guard ship** so the correction stays model-independent
on the record. Not a gold change until adjudicated.

| # | column | file(s) | current gold | defect (model-independent) | candidate | caveat |
|---|--------|---------|--------------|----------------------------|-----------|--------|
| 1 | `group_id:id` | `ir.model.access_{21896,95099,48768}.parquet` (3 rows) | `representation.identifier.alphanumeric_id` | Values are Odoo external IDs (`account.group_account_manager`, `base.user_root`) — **no digit**, so they fail alphanumeric_id's own validator (which requires one). | `technology.code.qualified_name` (nearest fit for `module.record` refs; the taxonomy has no "record reference" type) | **The candidate label agrees with the just-shipped `qualified_name_recovery` guard (17f481d) — MUST be adjudicated blind, not rubber-stamped to match the model.** Low urgency: 3 near-identical rows, +3 headline. Provenance: ac-04 two-panel 2026-06-10. |

Surfaced by: `qualified_name_recovery` finding (`output/qualified-name-recovery/finding.md`), 2026-07-13.

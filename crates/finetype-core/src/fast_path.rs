//! Deterministic `infer` fast-path — resolve a column's type from value-only
//! structural validators, *without* loading the neural model.
//!
//! `infer` classifies a single column; its dominant warm cost is the
//! multi-branch model load (~0.08s of a ~0.12s call — `infer-latency-breakdown`),
//! not the taxonomy load (~0.03s) or the classify (~0.02s). When a value is
//! *structurally* a known type — a `C:\…` path, a `<…@…>` message-id, an IPv4
//! address, an email, a delimited ISO timestamp — the model adds nothing: the
//! type is value-determinable (decision 0048). This module answers those without
//! the model, so the CLI can short-circuit before paying the load.
//!
//! ## Why only "conclusive" leaves
//!
//! The set below is deliberately narrow: each leaf's validator is *structurally
//! exclusive* — nothing else in the taxonomy passes the same value, and the
//! match is correct-by-construction (validator-passing ⟹ the value genuinely
//! *is* that type). On a multi-value sample the full Sense→Sharpen pipeline
//! lands the same leaf, so the fast-path is a pure latency win there. On a
//! *single* value it can additionally CORRECT the model: the neural classifier
//! is unreliable with one value and no header (e.g. it reads a lone IPv6 as an
//! integer, a lone MAC as a time), where the structural validator is right. So
//! this is a latency optimisation that is also single-value-robust for these
//! types — never a regression, because it only fires where the value provably
//! matches. The narrow set is the safety property: widening it to loose or
//! overlapping validators (closed-set/locale at large) would re-introduce the
//! context-free attractor failure mode the corpus-honest gate exists to catch,
//! so it is out of scope here.
//!
//! The match must be unanimous-enough on the *sample*, never on a single weak
//! signal; the threshold and the veto-consistency check mirror
//! `label_validates_sample` / `datetime_format_refinement` in the model crate so
//! the two stay a single source of truth.

use crate::taxonomy::Taxonomy;

/// Leaves whose validator passing is structurally conclusive — nothing else in
/// the taxonomy passes the same value, so the full pipeline lands here too.
/// Datetime is handled separately (it routes through the format detector, which
/// picks the precise ISO leaf). Keep this list conservative; every addition must
/// clear the fast-path-vs-full-pipeline agreement test before it ships.
const CONCLUSIVE_LEAVES: &[&str] = &[
    "technology.filesystem.windows_path", // drive-letter `C:\` / UNC `\\host\share`
    "technology.internet.message_id",     // RFC 2822 `<local@domain>`
    "technology.internet.ip_v4",
    "technology.internet.ip_v6",
    "technology.internet.ip_v4_with_port", // valid IPv4 + `:port`
    "technology.internet.cidr",            // valid IPv4 + `/0-32`
    "technology.internet.mac_address",
    "technology.internet.data_uri", // `data:…,…`
    "identity.person.email",
    "identity.person.email_display", // `Name <local@domain>`
    "identity.person.phone_e164",    // `+` then 7–15 digits
    // Structurally self-sufficient value leaves promoted from the model's
    // ceded-recovery set (spec 2026-06-27-model-label-space-reshape): each
    // validator is prefix/format/checksum-anchored, so a value passing it
    // provably IS that type and the full Sense→Sharpen pipeline lands here too.
    // The exactly-one-match gate below + the
    // `fast_path_never_mislabels_any_taxonomy_samples` test are the standing
    // exclusivity guards. Closed-set/locale leaves (day_of_week, month_name,
    // currency_code, http_method, gender, …), short/generic-shape leaves
    // (icd10, mime_type, si_number, ulid, vin, mgrs), and html (its `.*<tag.*`
    // validator matches prose containing a tag, and overlaps xml on closing-tag
    // markup) are DELIBERATELY excluded — they over-fire on non-type values
    // ("n/a" → mime_type, "B12" → icd10, "4K" → si_number) that the tiny
    // taxonomy sample set cannot surface.
    "representation.text.emoji",         // unicode symbol/mark run
    "representation.identifier.uuid",    // 8-4-4-4-12 hex
    "representation.format.color_hsl",   // `hsl(…)` / `hsla(…)`
    "representation.scientific.inchi",   // `InChI=1S/…` prefix
    "container.object.json",             // `{…}`
    "container.object.json_array",       // `[…]`
    "container.object.xml",              // `<…>…</…>`
    "finance.banking.iban",              // country + check digits + BBAN
    "finance.crypto.ethereum_address",   // `0x` + 40 hex
    "finance.currency.currency_symbol",  // pure currency-symbol run
    "finance.rate.basis_points",         // number + `bps`/`bp`
    "finance.securities.figi",           // BBG-style 12-char with pos-3 `G`
    "geography.coordinate.dms",          // `12°34'56"N …` degree/minute/second
    "geography.format.wkt",              // `POINT(…)` / `POLYGON(…)` …
    "geography.transportation.iso6346",  // container code `ABCU1234567`
    "technology.cloud.aws_arn",          // `arn:aws:…` prefix
    "technology.cloud.s3_uri",           // `s3://…` prefix
    "technology.code.doi",               // `10.NNNN/…` prefix
    "technology.cryptographic.jwt",      // three base64url segments
];

/// ≥90% of the sample's non-empty values must pass `leaf`'s validator. Mirrors
/// `label_validates_sample` (model crate) — the same bar the Sharpen
/// veto-consistency gate uses.
fn validates(tax: &Taxonomy, leaf: &str, sample: &[String]) -> bool {
    let mut checked = 0usize;
    let mut passed = 0usize;
    for v in sample {
        let t = v.trim();
        if t.is_empty() {
            continue;
        }
        if let Ok(res) = crate::validator::validate_value_for_label(t, leaf, tax) {
            checked += 1;
            if res.is_valid {
                passed += 1;
            }
        }
    }
    checked > 0 && (passed as f64) / (checked as f64) >= 0.9
}

/// Try to resolve a column's leaf from value-only structural validators, without
/// the neural model. Returns `Some(leaf)` only when the sample unambiguously
/// validates exactly one structurally-conclusive leaf — in which case the full
/// `infer` pipeline would produce the same answer, so the caller may emit it and
/// skip the model load. Returns `None` for anything that needs the model.
///
/// `tax` must have its validators compiled (`compile_validators`).
pub fn deterministic_fast_path(tax: &Taxonomy, sample: &[String]) -> Option<String> {
    if sample.iter().all(|v| v.trim().is_empty()) {
        return None;
    }

    // Delimited ISO datetimes are conclusive when the precise leaf's OWN
    // validator agrees (veto-consistency — same gate `datetime_format_refinement`
    // applies). A bare-integer datetime read is NOT taken here: the Sharpen rule
    // only accepts those with model corroboration, so the value alone can't.
    if let Some(d) = crate::datetime_format::detect_datetime_format(sample) {
        if d.delimited && validates(tax, d.leaf, sample) {
            return Some(d.leaf.to_string());
        }
    }

    // Exactly-one-match gate: resolve only when the sample validates a SINGLE
    // conclusive leaf. If two conclusive validators both pass (e.g. an ambiguous
    // markup snippet matching both html and xml), the value is not structurally
    // conclusive — abstain and let the model decide. This makes exclusivity a
    // property of the gate, not just of hand-curating a non-overlapping list.
    let mut matched: Option<&str> = None;
    for leaf in CONCLUSIVE_LEAVES {
        if validates(tax, leaf, sample) {
            if matched.is_some() {
                return None;
            }
            matched = Some(leaf);
        }
    }
    matched.map(|l| l.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taxonomy() -> Taxonomy {
        let labels_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("labels");
        let mut tax = Taxonomy::from_directory(&labels_dir).expect("load taxonomy");
        tax.compile_validators();
        tax
    }

    fn s(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn resolves_email() {
        let tax = taxonomy();
        assert_eq!(
            deterministic_fast_path(&tax, &s(&["jane.doe@company.co.uk", "bob@example.com"])),
            Some("identity.person.email".to_string())
        );
    }

    #[test]
    fn resolves_ipv4() {
        let tax = taxonomy();
        assert_eq!(
            deterministic_fast_path(&tax, &s(&["192.168.1.1", "10.0.0.1"])),
            Some("technology.internet.ip_v4".to_string())
        );
    }

    #[test]
    fn resolves_windows_path() {
        let tax = taxonomy();
        assert_eq!(
            deterministic_fast_path(&tax, &s(&[r"C:\Users\jane\file.txt", r"D:\data\x.csv"])),
            Some("technology.filesystem.windows_path".to_string())
        );
    }

    #[test]
    fn resolves_message_id() {
        let tax = taxonomy();
        assert_eq!(
            deterministic_fast_path(&tax, &s(&["<abc123@mail.example.com>", "<xyz@host.net>"])),
            Some("technology.internet.message_id".to_string())
        );
    }

    #[test]
    fn resolves_delimited_iso_datetime() {
        let tax = taxonomy();
        let got =
            deterministic_fast_path(&tax, &s(&["2024-01-15T14:30:00Z", "2024-02-20T09:00:00Z"]));
        assert!(
            got.as_deref()
                .map(|l| l.starts_with("datetime."))
                .unwrap_or(false),
            "expected a datetime leaf, got {got:?}"
        );
    }

    #[test]
    fn declines_ambiguous_short_code() {
        // A bare numeric/alnum code is not structurally conclusive — needs the model.
        let tax = taxonomy();
        assert_eq!(
            deterministic_fast_path(&tax, &s(&["90210", "10001", "60601"])),
            None
        );
        assert_eq!(
            deterministic_fast_path(&tax, &s(&["ABC123", "XYZ789"])),
            None
        );
    }

    #[test]
    fn declines_plain_words() {
        let tax = taxonomy();
        assert_eq!(
            deterministic_fast_path(&tax, &s(&["hello", "world", "foo"])),
            None
        );
    }

    #[test]
    fn declines_all_empty() {
        let tax = taxonomy();
        assert_eq!(deterministic_fast_path(&tax, &s(&["", "  "])), None);
    }

    fn samples_of(tax: &Taxonomy, leaf: &str) -> Vec<String> {
        tax.get(leaf)
            .map(|d| {
                d.samples
                    .iter()
                    .filter_map(|v| {
                        v.as_str().map(String::from).or_else(|| {
                            serde_yaml::to_string(v).ok().map(|x| x.trim().to_string())
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Standing exclusivity guard for `CONCLUSIVE_LEAVES` (the property the module
    /// doc calls "structurally exclusive"): for EVERY taxonomy type, the fast-path
    /// must return either `None` or that type's own leaf on its own samples — never
    /// a different conclusive leaf. A new conclusive leaf whose validator hijacks
    /// another type's values trips this. Value-only (no header), so no model needed.
    #[test]
    fn fast_path_never_mislabels_any_taxonomy_samples() {
        let tax = taxonomy();
        for (leaf, _) in tax.definitions() {
            let sample = samples_of(&tax, leaf);
            if sample.is_empty() {
                continue;
            }
            if let Some(got) = deterministic_fast_path(&tax, &sample) {
                assert_eq!(
                    &got, leaf,
                    "fast-path mislabelled {leaf}'s own samples as {got} — a conclusive \
                     leaf's validator is over-firing"
                );
            }
        }
    }

    #[test]
    fn resolves_promoted_value_leaves() {
        let tax = taxonomy();
        // Each is a value that provably IS its type; the model-only path was
        // unreliable on these single values (the reason for promotion).
        for (values, want) in [
            (vec!["❤️", "😀", "🎉"], "representation.text.emoji"),
            (
                vec![
                    "550e8400-e29b-41d4-a716-446655440000",
                    "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
                ],
                "representation.identifier.uuid",
            ),
            (vec!["InChI=1S/H2O/h1H2", "InChI=1S/CH4/h1H4"], "representation.scientific.inchi"),
            (vec!["POINT(30 10)", "LINESTRING(30 10, 10 30)"], "geography.format.wkt"),
            (
                vec![
                    "arn:aws:iam::123456789012:user/jane",
                    "arn:aws:s3:::my-bucket/key",
                ],
                "technology.cloud.aws_arn",
            ),
            (
                vec![
                    "0x52908400098527886E0F7030069857D2E4169EE7",
                    "0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe",
                ],
                "finance.crypto.ethereum_address",
            ),
        ] {
            assert_eq!(
                deterministic_fast_path(&tax, &s(&values)),
                Some(want.to_string()),
                "expected {want} for {values:?}"
            );
        }
    }
}

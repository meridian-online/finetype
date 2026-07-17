//! `guards.rs` — the deterministic post-Sense Sharpen guard layer:
//! the `apply_post_sharpen_guards` dispatcher and its substance-guard /
//! veto / recovery leaves, extracted from mod.rs (mechanical split, no
//! behaviour change). These attach to `ColumnClassifier` as a second impl.

use super::*;

/// Fraction of non-empty values carrying an organisation / investment-vehicle
/// suffix token — the value-side signal for `org_name_geography_demotion`. A place
/// name never carries one (US states / countries / regions / cities incl. `West
/// Bank` → 0.00), while a company/fund column is dense with them (gleif `name` →
/// 0.97). Deliberately DISTINCT from (and broader than) the entity classifier's
/// `org_suffixes` regex: that one is tuned narrow for person-vs-org demotion and
/// lacks the modern fund suffixes (`Fund`,`Capital`,`Trust`,`LP`,`Holdings`,…) that
/// dominate registry data, and it includes place-ambiguous tokens (`Bank`,`Co`,`SA`)
/// this guard must exclude to leave `West Bank`/`Cork` alone.
///
/// A match counts ONLY when the value has ≥2 whitespace tokens — i.e. the suffix is
/// a suffix OF a longer name (`DEUTZ AG`, `Kaanapali Land, LLC`), never the whole
/// value. This is load-bearing: several 2-letter company forms (`AB`,`AG`,`NV`,`SE`,
/// `BV`) are also US/Canada state codes and ISO country codes, so a bare-code `state`
/// column of `AB` (Alberta) / `country_code` of `SE` (Sweden) would otherwise match
/// at 100% and be wrongly demoted (corpus spot-check finding, mirrors the
/// constant-column lesson).
fn org_suffix_ratio(values: &[String]) -> f32 {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        // `\b`-delimited (the Rust regex crate has no look-around); calibrated
        // gleif `name` 0.97 vs every place / bare-geo-code category 0.00.
        regex::Regex::new(concat!(
            r"(?i)\b(Inc|Incorporated|LLC|LLP|LLLP|LP|Ltd|Limited|Corp|",
            r"Corporation|PLC|GmbH|AG|NV|BV|SE|OYJ|ASA|AB|Company|Fund|Funds|Trust|",
            r"Capital|Holdings|Holding|Partners|Partnership|Advisors|Advisers|",
            r"Management|Portfolios|Portfolio|Ventures|Associates|Securities|Insurance|",
            r"Investments|Investment|Bancorp|Bancshares|Group|Foundation|Enterprises|",
            r"Industries|Technologies|Solutions)\b",
        ))
        .expect("org_suffix_ratio regex")
    });
    let non_empty: Vec<&String> = values.iter().filter(|v| !v.trim().is_empty()).collect();
    if non_empty.is_empty() {
        return 0.0;
    }
    non_empty
        .iter()
        .filter(|v| v.split_whitespace().count() >= 2 && re.is_match(v))
        .count() as f32
        / non_empty.len() as f32
}

/// True when a value looks geographic — the value discriminator for
/// [`ColumnClassifier::region_nonmembership_veto`]. A value is a place if it is a
/// place NAME (`place_names`: GeoNames admin1 + countries + cities≥15k), an
/// ISO-3166-2 subdivision code (`US-TX`), a bare US/CA/AU state code (`CA`/`NV`),
/// or a `City, State` / `City (State)` composite whose `,`/`/`/`(`/`)`-delimited
/// parts are place names. Bare state codes are included at the value level so a
/// genuine state column stays, yet seismic `net` (a few state-named networks among
/// many non-geo codes) still falls below the veto's 50% column bar.
fn value_is_place(v: &str) -> bool {
    let t = v.trim();
    if t.is_empty() {
        return false;
    }
    if finetype_core::membership::place_names(t)
        || finetype_core::membership::iso_3166_2(t)
        || STATE_CODES.contains(&t.to_ascii_uppercase().as_str())
    {
        return true;
    }
    // Composite `City, State` / `City (State)` — any delimited part is a place name.
    t.split([',', '/', '(', ')'])
        .any(|part| finetype_core::membership::place_names(part.trim()))
}

impl ColumnClassifier {
    /// Guards that must fire on the POST-sharpen label — including labels a
    /// header hint created via an early `return` inside `apply_header_sharpen`.
    /// Every value-identical-boundary guard whose target a hint can synthesise
    /// belongs here, where it runs unconditionally, not inside
    /// `apply_header_sharpen` where the hint branches would make it unreachable.
    pub(crate) fn apply_post_sharpen_guards(
        &self,
        result: &mut ColumnResult,
        header: &str,
        sample: &[String],
        values: &[String],
    ) {
        self.amount_bare_number_veto(result, header, sample);
        self.url_bare_number_veto(result, header, sample);
        self.utc_bare_number_veto(result, header, sample);
        self.checksum_substance_guard(result, header, sample);
        self.membership_substance_guard(result, header, sample);
        self.isbn_header_recovery(result, header, sample);
        self.binary_vocab_veto(result, header, sample, values);
        self.increment_substance_veto(result, values);
        // AFTER increment_substance_veto: the sequential-detection promotion
        // routes code columns through `increment`, and the veto lands them on
        // integer_number — this recovery must see that final integer, not the
        // intermediate increment.
        self.numeric_code_header_recovery(result, header, sample);
        self.unlocode_format_veto(result, sample);
        // Grouped with the other geo-shape overcall demotes: a `legal_form`/`elf`
        // column of 4-digit ISO-20275 sentinels reads as a numeric postal_code;
        // demote off the false postal claim to the `word` residual (no clean type
        // to promote to — the column is 91% "no legal form").
        self.legal_form_postal_demote(result, header);
        self.city_region_header_corroboration(result, header, sample);
        self.country_code_corroboration(result, header, sample);
        self.geo_code_membership_vote(result, header, sample);
        self.geo_code_nonmembership_demotion(result, header, sample);
        self.geo_subdivision_membership_promote(result, header, sample);
        // AFTER the geo-membership promotes (a real ISO subdivision has 0% org
        // suffix, so it is never in scope): demote an org-name column that the
        // model mistyped as a place (gleif `name`->region) to entity_name.
        self.org_name_geography_demotion(result, sample);
        // AFTER the geo promotes AND org-name demotion: a real ISO subdivision
        // (US-TX) is protected by iso_3166_2 membership and a company name is
        // already entity_name, so this veto only sees a residual `region` overcall
        // on a non-place column (usgs net/type, gleif category).
        self.region_nonmembership_veto(result, header, sample);
        // AFTER org_name_geography_demotion AND region_nonmembership_veto: the
        // legit place→entity_name promote needs org_suffix_ratio >= 0.5, which
        // this veto's < 0.1 gate structurally excludes — ordering keeps that
        // promote's output out of this veto's scope by construction.
        self.entity_name_vocab_veto(result, sample);
        // AFTER geo_subdivision_membership_promote: keeps the geo-membership
        // promotes grouped. UN/LOCODE (`USLAX`) is hyphenless, so the ISO-3166-2
        // promote above never claims it — no fight.
        self.unlocode_membership_recovery(result, sample);
        self.timezone_abbreviation_recovery(result, header, sample);
        self.naics_industry_recovery(result, header, sample);
        // ticker: header-gated membership promote — recovers finance.securities.ticker
        // from the state_code / word attractor (EDGAR ticker external-band finding).
        self.ticker_membership_recovery(result, header, sample);
        // tld: header-gated membership promote — recovers top_level_domain from the
        // continent overcall (majestic TLD external-band finding).
        self.tld_geography_recovery(result, header, sample);
        // Header-gated code recoveries grouped with naics: CPT and HS have no
        // check digit and their bare shapes are value-identical with ZIP / a
        // plain integer, so the header token is the sole discriminator.
        self.cpt_procedure_recovery(result, header, sample);
        self.hs_code_header_recovery(result, header, sample);
        // IMEI: 15-digit Luhn is NOT self-precise (a 15-digit Amex card is
        // Luhn-valid by construction), so this one is header-gated too.
        self.imei_checksum_recovery(result, header, sample);
        self.s_expression_recovery(result, sample);
        self.qualified_name_recovery(result, sample);
        self.filename_recovery(result, sample);
        self.delimited_array_recovery(result, sample);
        self.version_string_recovery(result, header, sample);
        // color_rgb: anchored `rgb(`/`rgba(` prefix, self-precise like s_expression.
        self.color_rgb_recovery(result, sample);
        self.ceded_leaf_recovery(result, sample);
        // AFTER ceded_leaf_recovery: ISIN and ISRC share a 12-char shape, so the
        // regex-only ceded recovery mislabels digit-tailed ISINs as isrc. This
        // corrects that using the ISIN check digit (which the regex validator
        // cannot see), so it must run last to override the isrc misassertion.
        self.isin_checksum_recovery(result, sample);
        // Value-only checksum recoveries grouped with isin: CUSIP / SEDOL / DEA
        // each land on a value-identical id attractor (word / numeric_code /
        // alphanumeric_id), and their scheme-specific check digit — which the
        // regex-only ceded recovery cannot see — is the self-precise discriminator,
        // so no header gate is needed and they round-trip headerless.
        self.cusip_checksum_recovery(result, sample);
        self.sedol_checksum_recovery(result, sample);
        self.dea_checksum_recovery(result, sample);
        // LAST: the recovery guards above get first crack at relocating a
        // wrongly-jwt/mime column to a real type; these substance guards then demote
        // to `unknown` only what remains stubbornly labelled jwt/mime_type, and
        // nothing after can re-promote it via the shape-only validator.
        self.mime_type_substance_guard(result, sample);
        self.locale_code_substance_guard(result, sample);
        self.password_substance_guard(result, sample);
        self.jwt_substance_guard(result, sample);
    }

    /// `timezone_abbreviation_recovery` (default ON). Recovers the
    /// `datetime.offset.timezone_abbreviation` leaf (spec
    /// 2026-06-25-timezone-abbreviation-type). The 240-dim model cannot predict
    /// this mined leaf, and the header-hint machinery routes a `timezone` header
    /// to `datetime.offset.iana` — but a column of bare abbreviations (EDT/CEST)
    /// is not an IANA zone name (Region/City). Promote a residual / iana label to
    /// the abbreviation leaf when the values pass its closed-set validator AND the
    /// header corroborates a timezone column. The header gate is load-bearing:
    /// EST/CST/PST overlap estimate/cost, so the abbreviation set alone over-emits
    /// (corpus: 1,442/1,444 tz columns are uppercase, but a non-tz "EST" status
    /// column would still match the set — the header is what excludes it).
    /// Value-based (0048), RHH-disableable. CORROBORATION + veto-consistency gated
    /// like `structured_string_refinement`.
    fn timezone_abbreviation_recovery(
        &self,
        result: &mut ColumnResult,
        header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("timezone_abbreviation_recovery") {
            return;
        }
        const LEAF: &str = "datetime.offset.timezone_abbreviation";
        // Precision rests on TWO gates, NOT a source-label allowlist: the header
        // must name a timezone column AND >=90% of values must pass the closed-set
        // validator. The model funnels uppercase abbreviations into 3-letter-code
        // attractors (country_code/iata_code) as readily as into a residual, so an
        // allowlist misses them — but a genuine country_code/iata column fails the
        // tz validator (US/LHR are not tz abbreviations) and carries no timezone
        // header, so neither gate admits it. Only the leaf itself is excluded.
        if result.label == LEAF || !header_corroborates_timezone(header) {
            return;
        }
        match self.taxonomy.as_ref() {
            Some(tax) if label_validates_sample(tax, LEAF, sample) => {
                result.label = LEAF.to_string();
                result.confidence = result.confidence.max(0.95);
                result.disambiguation_applied = true;
                result.disambiguation_rule = Some(format!(
                    "timezone_abbreviation_recovery:{}",
                    header.to_lowercase()
                ));
                result.detected_locale = None;
            }
            _ => {}
        }
    }

    /// `naics_industry_recovery` (default ON). Recovers the
    /// `identity.industry.naics` leaf (company-reference audit W3). The shipped
    /// 244-dim model cannot predict this leaf, and even its residual home
    /// (`numeric_code`) needs header rescue from the F5 leading-zero demotion —
    /// so a NAICS column reaches this guard as integer_number or numeric_code.
    /// Promote when the header names a NAICS column (`header_corroborates_naics`
    /// — the distinctive `naics` token; `sic` and bare `industry` deliberately
    /// excluded) AND ≥90% of values are members of the published Census code
    /// list (`membership::naics_codes`, labels/sets/naics_codes.txt). The
    /// two-gate discipline mirrors `timezone_abbreviation_recovery`: sector
    /// codes are value-identical with small integers, so the header gate is
    /// load-bearing; a quantity column fails the header gate, a naics-headed
    /// text column fails membership. Value-based (0048), RHH-disableable.
    fn naics_industry_recovery(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("naics_industry_recovery") {
            return;
        }
        const LEAF: &str = "identity.industry.naics";
        if result.label == LEAF {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        // Header gate, two tiers: the distinctive `naics` token admits any code
        // level, while a generic code-ish header (`code`, the real product
        // surface's bare header) is admitted only for >=4-digit codes — the
        // 2,129-code set makes 6-digit coincidence ~0.1%, but 2-digit sectors
        // span 11-92, which a rating/age-like column headed `code` could
        // fully occupy by accident.
        let mut lens: Vec<usize> = non_empty.iter().map(|v| v.len()).collect();
        lens.sort_unstable();
        let median_len = lens[lens.len() / 2];
        if !(header_corroborates_naics(header)
            || (header_corroborates_numeric_code(header) && median_len >= 4))
        {
            return;
        }
        let members = non_empty
            .iter()
            .filter(|v| finetype_core::membership::naics_codes(v))
            .count();
        // >=90% in the published code list (members*10 >= len*9).
        if members * 10 < non_empty.len() * 9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.95);
        result.disambiguation_applied = true;
        result.disambiguation_rule =
            Some(format!("naics_industry_recovery:{}", header.to_lowercase()));
        result.detected_locale = None;
    }

    /// `ticker_membership_recovery` (default ON). Recovers the
    /// `finance.securities.ticker` leaf (company-reference external band: EDGAR
    /// `ticker` was over-emitted as `geography.location.state_code`). The
    /// 244-dim model cannot predict this leaf, and a short uppercase symbol
    /// column lands on state_code / word / alphanumeric_id. Promote when the
    /// header names a ticker column (`header_corroborates_ticker` — `ticker` /
    /// `symbol`) AND ≥90% of values are US-listed symbols
    /// (`membership::us_tickers`, labels/sets/us_tickers.txt) AND ≥3 DISTINCT
    /// pass. The header gate is load-bearing: 15 of the 50 US state codes (`MA`,
    /// `TX`, …) are themselves real tickers, so membership alone cannot separate
    /// a ticker column from a state column — the header is the discriminator. The
    /// ≥3-distinct gate blocks a constant column matching one symbol by
    /// coincidence (the `unlocode_membership_recovery` constant-column lesson).
    /// Value-based (0048), RHH-disableable.
    fn ticker_membership_recovery(
        &self,
        result: &mut ColumnResult,
        header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("ticker_membership_recovery") {
            return;
        }
        const LEAF: &str = "finance.securities.ticker";
        if result.label == LEAF || !header_corroborates_ticker(header) {
            return;
        }
        let mut checked = 0usize;
        let mut passed = 0usize;
        let mut distinct_pass: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for v in sample {
            let t = v.trim();
            if t.is_empty() {
                continue;
            }
            checked += 1;
            if finetype_core::membership::us_tickers(t) {
                passed += 1;
                distinct_pass.insert(t);
            }
        }
        // >=90% US-listed membership AND >=3 DISTINCT passing values.
        if checked < 3 || distinct_pass.len() < 3 || passed * 10 < checked * 9 {
            return;
        }
        let from = result.label.clone();
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.90);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("ticker_membership_recovery:{from}"));
        result.detected_locale = None;
    }

    /// `org_name_geography_demotion` (default ON). Demotes a geography overcall on
    /// an organisation-name column to `representation.text.entity_name`
    /// (company-reference external band, seam 1c: gleif `name` → region). The raw
    /// Sense model reaches for a place when it sees proper-noun text, so an org-name
    /// column lands on region/city/country. The value-side tell is self-precise: a
    /// company name carries an org/fund suffix (`… PLC`, `… Fund`, `… Capital`,
    /// `… LP`) and a place name never does — measured gleif `name` 0.97, US-state /
    /// country / region / city columns 0.00 (incl. `West Bank`, since `Bank`/`Co`/`SA`
    /// are deliberately excluded from the suffix set as place-ambiguous). Demote when
    /// ≥50% of values carry an org suffix (`org_suffix_ratio`) AND the label is a
    /// place-NAME leaf. No header gate: the suffix signal is self-precise, so it corrects
    /// an org column even under a generic `name`/`value` header (where a header veto could
    /// not). Value-based (0048), RHH-disableable.
    ///
    /// Scope is the place-NAME leaves only (`city`/`region`/`country`/`continent`) — NOT
    /// the address leaves (`geography.address.*`). A street address is legitimately
    /// multi-word free text carrying directional/building tokens that collide with org
    /// suffixes: `4th Street SE` (SE = South-East, not the Societas-Europaea form),
    /// `Royal Trust Tower` (a building, not a trust), `Bairro Asa` (a Brasília district,
    /// not the Norwegian ASA form). Those are 100% of the observed false positives and
    /// they live entirely in address columns; the seam itself (name → region/city) never
    /// touches an address leaf, so gating on place-name leaves removes the whole FP class
    /// structurally rather than by chasing false-friend tokens. Bare code leaves
    /// (`country_code`/`state_code`) are also out of scope — a code column carries no org
    /// suffix and must never become an entity name.
    fn org_name_geography_demotion(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("org_name_geography_demotion") {
            return;
        }
        const LEAF: &str = "representation.text.entity_name";
        const PLACE_NAME_LEAVES: [&str; 4] = [
            "geography.location.city",
            "geography.location.region",
            "geography.location.country",
            "geography.location.continent",
        ];
        if !PLACE_NAME_LEAVES.contains(&result.label.as_str()) {
            return;
        }
        // >=50% org-suffix — huge margin below the 0.97 org-column rate and above the
        // 0.0 place-column rate; a geography leaf never legitimately clears it.
        if org_suffix_ratio(sample) < 0.5 {
            return;
        }
        let from = result.label.clone();
        result.label = LEAF.to_string();
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("org_name_geography_demotion:{from}"));
        result.detected_locale = None;
    }

    /// `region_nonmembership_veto` (default ON). Demotes a `geography.location.region`
    /// OVERCALL to `representation.text.word` when a column's distinct values are mostly
    /// NOT real places — the tier-3 geography seam (external band). The raw Sense model
    /// treats `region` as a garbage-magnet for short catalog codes and enum words: usgs
    /// `net`/`type`/`magSource`/`locationSource` (seismic network + event codes), gleif
    /// `category` (`GENERAL`/`FUND`), seattle `checkouttype` (`Horizon`/`OverDrive`), nyc
    /// `permit_subtype`. A value counts as a place (`value_is_place`) if it is a place NAME
    /// (`membership::place_names` — GeoNames admin1 + countries + cities≥15k), an ISO-3166-2
    /// subdivision code (`US-TX`, so a code-valued `iso_region` column stays), a bare state
    /// code (`CA`/`NV`), or a `City, State` / `City (State)` composite whose parts are
    /// places. Cities and composites are load-bearing: an admin1-only gazetteer wrongly
    /// demoted real city/county columns (`Austin`, `Durham County, NC`) at a measured 15%
    /// false-positive rate — the 33k spot-check that caught it is exactly the discipline the
    /// gate (oracle-blind here) cannot provide. Demote when < 50% of distinct values are
    /// places AND the header is not a strong region/state header. A genuine region/city/
    /// county column clears ~0.9; the false catalog columns clear ~0.0 (seismic `net` sits
    /// at ~0.33 — a few networks are named after states — safely below the bar). Value-based
    /// (0048), RHH-disableable.
    fn region_nonmembership_veto(
        &self,
        result: &mut ColumnResult,
        header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("region_nonmembership_veto") {
            return;
        }
        const LEAF: &str = "representation.text.word";
        if result.label != "geography.location.region" {
            return;
        }
        // A strong region/state header protects a genuine subdivision column whose
        // names the gazetteer may miss (obscure / non-English admin1 divisions).
        if header_corroborates_region(header) || header_corroborates_state(header) {
            return;
        }
        // A bare-CODE subdivision column (state=`AB`/`NV`, which a sibling guard
        // normalises state_code->region) is geographic but scores 0% on the
        // names+hyphenated-codes gazetteer. `values_look_like_state_codes` (>=80%
        // STATE_CODES) keeps it — and does NOT rescue the false columns: usgs `net`
        // is `us`-dominated (a country code, not a state code), so it stays <80%.
        if values_look_like_state_codes(sample) {
            return;
        }
        let mut distinct: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for v in sample {
            let t = v.trim();
            if !t.is_empty() {
                distinct.insert(t);
            }
        }
        // >=2 distinct: a single value carries too little signal to overturn the
        // model, and a constant real-region column (a dataset all in one state) must
        // not be vetoed on one gazetteer miss.
        if distinct.len() < 2 {
            return;
        }
        let places = distinct.iter().filter(|v| value_is_place(v)).count();
        if places * 2 >= distinct.len() {
            return;
        }
        let from = result.label.clone();
        result.label = LEAF.to_string();
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("region_nonmembership_veto:{from}"));
        result.detected_locale = None;
    }

    /// `entity_name_vocab_veto` (default ON). Demotes a
    /// `representation.text.entity_name` OVERCALL on a low-cardinality
    /// single-token controlled vocabulary to `representation.text.word` — the
    /// established residual sink for small enums. `entity_name` is a
    /// `broad_words` catch-all with no validation pattern, and it is absent
    /// from `TEXT_ATTRACTORS`, so neither the validation veto nor the
    /// attractor demotions ever evaluate it: a 5-value enum column
    /// (`sector`/`subsector`/`industry_group`/…) asserted entity_name at low
    /// confidence ships uncorrected. The value-side tell is shape: a genuine
    /// entity-name column is dominated by multi-word names (`Toyota Motor
    /// Corp`) or carries org/fund suffixes, while a controlled vocabulary is a
    /// bounded set of single tokens. Demote when the sample has ≥3 non-empty
    /// values, ≤20 distinct values, `org_suffix_ratio < 0.1` (excludes genuine
    /// org columns — including everything `org_name_geography_demotion`
    /// promotes, which needs ≥0.5), and ≥90% of distinct values contain no
    /// internal whitespace (single-token `word` shape — the gate that
    /// structurally spares multi-word entity names). Known trade: a
    /// low-cardinality SINGLE-token Title-Case brand vocabulary would also
    /// demote; gold holds no such column. Value-based (0048), demote-only,
    /// RHH-disableable. Modeled on `region_nonmembership_veto`.
    fn entity_name_vocab_veto(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("entity_name_vocab_veto") {
            return;
        }
        const LEAF: &str = "representation.text.word";
        if result.label != "representation.text.entity_name" {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        // ≥3 values: too few carries no distributional evidence to overturn
        // the model (the constant-column lesson, mirrored from the membership
        // recoveries).
        if non_empty.len() < 3 {
            return;
        }
        let distinct: std::collections::HashSet<&str> = non_empty.iter().copied().collect();
        // ≤20 distinct: the controlled-vocabulary bound (mirrors the attractor
        // cardinality signal and R32's enum band).
        if distinct.len() > 20 {
            return;
        }
        // A genuine org-name column is dense with org/fund suffixes (gleif
        // `name` 0.97); a controlled vocabulary carries ~none. <0.1 keeps every
        // real org column — and the org_name_geography_demotion promote
        // (≥0.5) — structurally out of scope.
        if org_suffix_ratio(sample) >= 0.1 {
            return;
        }
        // ≥90% of DISTINCT values single-token (no internal whitespace): the
        // `word` shape. Multi-word entity names (`Toyota Motor Corp`) fail
        // this gate, so a real name column never demotes.
        let single_token = distinct
            .iter()
            .filter(|v| !v.chars().any(char::is_whitespace))
            .count();
        if single_token * 10 < distinct.len() * 9 {
            return;
        }
        let from = result.label.clone();
        result.label = LEAF.to_string();
        result.confidence = result.confidence.min(0.6);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("entity_name_vocab_veto:{from}"));
        result.detected_locale = None;
    }

    /// `tld_geography_recovery` (default ON). Recovers `technology.internet.top_level_domain`
    /// from a `geography.location.continent` (or other geo) overcall — the company-reference
    /// external band's TLD→continent miss (majestic-million `TLD`/`IDN_TLD`). The raw Sense
    /// model, seeing short lowercase tokens (`com`,`org`,`uk`), reaches for a place. Promote
    /// when the header names a TLD column (`header_corroborates_tld`) AND ≥90% of values are
    /// IANA-delegated TLDs (`membership::tld_codes`) AND ≥3 DISTINCT pass. The header gate is
    /// load-bearing: a pure-ccTLD column is value-identical to a country-code column, so
    /// membership alone over-promotes; the `tld` header marks it as domains. Value-based
    /// (0048), RHH-disableable.
    fn tld_geography_recovery(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("tld_geography_recovery") {
            return;
        }
        const LEAF: &str = "technology.internet.top_level_domain";
        if result.label == LEAF || !header_corroborates_tld(header) {
            return;
        }
        let mut checked = 0usize;
        let mut passed = 0usize;
        let mut distinct_pass: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for v in sample {
            let t = v.trim();
            if t.is_empty() {
                continue;
            }
            checked += 1;
            if finetype_core::membership::tld_codes(t) {
                passed += 1;
                distinct_pass.insert(t);
            }
        }
        if checked < 3 || distinct_pass.len() < 3 || passed * 10 < checked * 9 {
            return;
        }
        let from = result.label.clone();
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.90);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("tld_geography_recovery:{from}"));
        result.detected_locale = None;
    }

    /// `s_expression_recovery` (default ON). Recovers the
    /// `container.object.s_expression` leaf (company-reference audit; author-
    /// approved 2026-07-05 as a general S-expression type over a narrow
    /// parse_tree leaf). A parse tree's Penn comma-tokens `(, ,)` fool the
    /// delimiter detector into reading a CSV array, so 1,292 corpus columns of
    /// constituency parses / code ASTs (`trees`/`parse_tree`/`ast`) land on
    /// `container.array.comma_separated`. The 244-dim model cannot predict the
    /// leaf. Promote when >=90% of values pass the balanced-nested-paren
    /// structural check (`finetype_core::structure::is_s_expression`). NO header
    /// gate — the structural signature is self-precise (measured corpus
    /// over-recovery: zero, 1,292/1,292 were genuine S-expressions), unlike the
    /// value-ambiguous checksum/membership recoveries. Value-based (0048),
    /// RHH-disableable.
    fn s_expression_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("s_expression_recovery") {
            return;
        }
        const LEAF: &str = "container.object.s_expression";
        if result.label == LEAF {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        let valid = non_empty
            .iter()
            .filter(|v| finetype_core::structure::is_s_expression(v))
            .count();
        // >=90% pass the balanced-nested-paren structural check.
        if valid * 10 < non_empty.len() * 9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.95);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("s_expression_recovery".to_string());
        result.detected_locale = None;
    }

    /// `qualified_name_recovery` (default ON). Recovers
    /// `technology.code.qualified_name` (a dotted namespaced code symbol) that the
    /// 244-dim model cannot predict, leaving ~1,300 corpus columns of .NET/Java
    /// namespaces (`ICSharpCode.NRefactory6`, `Abot2.Tests.Integration`,
    /// `AgileWizard.Domain`) misfiled. Residual audit (2026-07-13);
    /// `structured_string_refinement` already recovers the 3+-segment forms FROM
    /// residual via the taxonomy validator (`{2,}` = three-or-more segments), but that
    /// validator rejects the 2-segment forms AND must NOT be widened — a bare `foo.bar`
    /// is too common (Precision Principle). This guard closes both gaps with two tiers,
    /// each keyed to a corpus-measured live-Sense distribution:
    ///
    /// - **Tier 1 (residual → qn):** `plain_text`/`word`/`unknown` promoted on
    ///   `is_qualified_name` (2-segment needs a code signal + not-a-filename; 3+ direct).
    /// - **Tier 2 (confident-mislabel override → qn):** the name/place/host text labels
    ///   the model actively reaches for on a dotted PascalCase token
    ///   (`entity_name`/`hostname`/`full_name`/`full_address`/`city`/`region`) promoted
    ///   on the stricter `is_qualified_name_strong` — a code signal AND not a canonical
    ///   hostname, so a genuine `www.breitbart.com` (Sense=hostname) is spared. Measured:
    ///   zero real-host false positives. `username`/`alphanumeric_id` are deliberately
    ///   EXCLUDED (Odoo user-refs `base.user_root`, HDF5 filenames `…_bf.h5` overlap).
    ///
    /// Promote when >=90% of non-empty values pass the tier's detector. NO header gate
    /// (the detectors are self-precise). Value-based (0048), RHH-disableable.
    fn qualified_name_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("qualified_name_recovery") {
            return;
        }
        const LEAF: &str = "technology.code.qualified_name";
        const RESIDUAL: &[&str] = &[
            "representation.text.plain_text",
            "representation.text.word",
            "unknown",
        ];
        const OVERRIDE: &[&str] = &[
            "representation.text.entity_name",
            "technology.internet.hostname",
            "identity.person.full_name",
            "geography.address.full_address",
            "geography.location.city",
            "geography.location.region",
        ];
        let label = result.label.as_str();
        let detector: fn(&str) -> bool = if RESIDUAL.contains(&label) {
            finetype_core::structure::is_qualified_name
        } else if OVERRIDE.contains(&label) {
            finetype_core::structure::is_qualified_name_strong
        } else {
            return;
        };
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        let valid = non_empty.iter().filter(|v| detector(v)).count();
        if valid * 10 < non_empty.len() * 9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.95);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("qualified_name_recovery".to_string());
        result.detected_locale = None;
    }

    /// `filename_recovery` (default ON). Recovers `technology.filesystem.filename` (a bare
    /// file name — stem + real extension, no directory), minted 2026-07-14 as a sibling of
    /// `windows_path` off the reservoir-mining sweep: ~850 corpus columns of filenames
    /// (`report_final.xlsx`, `L2M-*.pdf`, MAME `.cpp` sources) sprayed across confident-wrong
    /// buckets. The 244-dim model cannot predict the leaf; there is no prior recovery
    /// (`structure::FILE_EXTENSIONS` existed only to EXCLUDE filenames from qualified_name).
    ///
    /// Fires on the RESIDUAL labels plus the measured confident-mislabel OVERRIDE set
    /// (entity_name / alphanumeric_id / token_urlsafe / version / full_address / full_name /
    /// bitcoin_address / jwt / isbn / username). `hostname` and `url` are DELIBERATELY EXCLUDED
    /// — a bare ccTLD domain (`gov.md`) is shape-identical to a markdown file, so overriding a
    /// confident locator would create false positives. Promote when >=90% of values pass
    /// `finetype_core::structure::is_filename` AND >=3 distinct pass (a near-constant column
    /// carries too little signal to override a foreign prediction). Value-based (0048),
    /// RHH-disableable, NO retrain (0096).
    fn filename_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("filename_recovery") {
            return;
        }
        const LEAF: &str = "technology.filesystem.filename";
        const FIRE_ON: &[&str] = &[
            "representation.text.plain_text",
            "representation.text.word",
            "unknown",
            "representation.text.entity_name",
            "representation.identifier.alphanumeric_id",
            "technology.cryptographic.token_urlsafe",
            "technology.development.version",
            "geography.address.full_address",
            "identity.person.full_name",
            "finance.crypto.bitcoin_address",
            "technology.cryptographic.jwt",
            "identity.commerce.isbn",
            "identity.person.username",
        ];
        if !FIRE_ON.contains(&result.label.as_str()) {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        let mut distinct_pass: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut passed = 0usize;
        for v in &non_empty {
            let t = v.trim();
            if finetype_core::structure::is_filename(t) {
                passed += 1;
                distinct_pass.insert(t);
            }
        }
        if passed * 10 < non_empty.len() * 9 || distinct_pass.len() < 3 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.95);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("filename_recovery".to_string());
        result.detected_locale = None;
    }

    /// `delimited_array_recovery` (default ON). Recovers the `container.array.*`
    /// leaves (comma / pipe / semicolon) that the 244-dim model strands as
    /// residual or entity text — ~470 corpus columns of genuine delimited lists
    /// (`[20000, 10000, 15000]`, `Biography|Comedy|Drama`, `subjects: polymers;raman`)
    /// misfiled as plain_text / word / unknown / entity_name (reservoir-mining
    /// sweep, 2026-07-14).
    ///
    /// The substance check (`finetype_core::structure::delimited_list_delim`) is
    /// self-precise by construction: it accepts a comma list ONLY when brackets
    /// disambiguate it (`[a, b, c]`), and otherwise only the pipe / semicolon
    /// delimiters — which, unlike the bare comma, never live inside a date, money
    /// figure, decimal, address, or place name. The bare-comma majority of the
    /// reservoir (`Winter Park, Florida`, single addresses, author lists) is
    /// deliberately NOT recovered: a comma between two words is structurally
    /// identical whether it is a list separator or an intra-entity comma, so it
    /// cannot be told from a `city`/`full_address` by value alone (Precision
    /// Principle).
    ///
    /// FIRE_ON is the residual set plus `entity_name` — the labels where a
    /// delimited-list value is unambiguously a mislabel. The numeric-sense
    /// overrides (`coordinate`, `currency.amount_comma`) are held back: a bracketed
    /// two-element numeric list carries a genuine coordinate/decimal ambiguity that
    /// needs its own element-count carve-out (deferred follow-up).
    ///
    /// Per-column delimiter **voting**: each cell votes its delimiter, the winner
    /// must carry >=90% of the passing cells (column coherence), and its
    /// `container.array.<delim>_separated` leaf is assigned. Promote when >=90% of
    /// non-empty values pass AND >=3 distinct values pass the winning delimiter (a
    /// near-constant column carries too little signal to override a foreign
    /// prediction). Value-based (0048), RHH-disableable, NO retrain (0096).
    fn delimited_array_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("delimited_array_recovery") {
            return;
        }
        const FIRE_ON: &[&str] = &[
            "representation.text.plain_text",
            "representation.text.word",
            "unknown",
            "representation.text.entity_name",
        ];
        if !FIRE_ON.contains(&result.label.as_str()) {
            return;
        }
        const LEAVES: [&str; 3] = [
            "container.array.comma_separated",
            "container.array.pipe_separated",
            "container.array.semicolon_separated",
        ];
        use finetype_core::structure::ListDelim;
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        let mut votes = [0usize; 3];
        let mut distinct: [std::collections::HashSet<&str>; 3] = Default::default();
        let mut passed = 0usize;
        for v in &non_empty {
            let t = v.trim();
            if let Some(d) = finetype_core::structure::delimited_list_delim(t) {
                let i = match d {
                    ListDelim::Comma => 0,
                    ListDelim::Pipe => 1,
                    ListDelim::Semicolon => 2,
                };
                votes[i] += 1;
                distinct[i].insert(t);
                passed += 1;
            }
        }
        if passed * 10 < non_empty.len() * 9 {
            return;
        }
        let win = (0..3).max_by_key(|&i| votes[i]).unwrap();
        // the winning delimiter must dominate the passing cells (column coherence)
        if votes[win] * 10 < passed * 9 || distinct[win].len() < 3 {
            return;
        }
        result.label = LEAVES[win].to_string();
        result.confidence = result.confidence.max(0.95);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("delimited_array_recovery".to_string());
        result.detected_locale = None;
    }

    /// `version_string_recovery` (default ON). Recovers `technology.development.version`
    /// (a `v?MAJOR.MINOR.PATCH` software version) that the 244-dim model strands as
    /// `unknown` / residual text — ~370 corpus columns of `ver` / `build` / `Fix Version`
    /// values (`1.6.1`, `1.11.23`, `1.2.2`) the model never learned to call a version
    /// (reservoir-mining sweep, 2026-07-14).
    ///
    /// This is the sweep's one HEADER-GATED recovery, and the gate is LOAD-BEARING, not
    /// corroboration: the version shape is value-AMBIGUOUS (a `YYYY.MM.DD` date and a
    /// `YYYY.MM.PATCH` calver share the three-dotted-number shape), so value alone
    /// over-promotes (measured: the header gate cuts the shaped reservoir 570 → 372,
    /// removing 20 date columns). Fires ONLY when `header_corroborates_version` (a
    /// `version`/`ver`/`build`/… token, no date token) AND ≥90% of values pass
    /// `finetype_core::structure::is_version_string` (SemVer shape + the four-digit-year
    /// veto that excludes dates/calver).
    ///
    /// Fires on the residual labels PLUS the two numeric labels the model reaches for
    /// on a dotted `N.N.N`: `value_sharpen`'s feature rule promotes the raw-model
    /// `unknown` to `integer_number` / `decimal_number` (a version looks float-ish)
    /// BEFORE this guard runs, and the validation veto only knocks it back to `unknown`
    /// AFTER — so at guard time the label is numeric, not residual. Firing on the numeric
    /// pair is safe: `is_version_string` demands exactly three dotted components with the
    /// year veto, so no genuine integer (`42`) or decimal (`3.14`) can pass, and the
    /// load-bearing version header gates it further. A confident date leaf (`dmy_short_dot`,
    /// `ymd_dot`) is deliberately NOT included — that value-ambiguous boundary is
    /// `value_sharpen`'s Rule 31 job (impossible-date-segment demotion), and overriding a
    /// confident date on a header alone is the mistake the exclusion avoids. NO
    /// distinct-cardinality floor: a constant version column (`1.6.1` on every row) is
    /// normal and correct, so the header gate — not diversity — is the precision.
    /// Value-based (0048), RHH-disableable, NO retrain (0096).
    fn version_string_recovery(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("version_string_recovery") {
            return;
        }
        const LEAF: &str = "technology.development.version";
        const FIRE_ON: &[&str] = &[
            "unknown",
            "representation.text.plain_text",
            "representation.text.word",
            "representation.numeric.integer_number",
            "representation.numeric.decimal_number",
        ];
        if !FIRE_ON.contains(&result.label.as_str()) || !header_corroborates_version(header) {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        let passed = non_empty
            .iter()
            .filter(|v| finetype_core::structure::is_version_string(v))
            .count();
        if passed * 10 < non_empty.len() * 9 {
            return;
        }
        let from = result.label.clone();
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.9);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("version_string_recovery:{from}"));
        result.detected_locale = None;
    }

    /// `jwt_substance_guard` (default ON). Demotes `technology.cryptographic.jwt`
    /// to `unknown` when the column's values are not real JWTs. The taxonomy
    /// pattern checks only the three-base64url-segment SHAPE, so the model
    /// over-emits jwt on text at corpus scale (7,920 of the 33k-sample columns —
    /// Windows file paths, prose, entity names; the gated-YDF oracle refutes
    /// every one, 0 confirmed jwt). A genuine JWT header base64url-decodes to a
    /// JSON object with an `alg` key (`finetype_core::structure::is_jwt`); when
    /// fewer than half a column's values carry that certainty the jwt assertion
    /// is wrong. We demote to `unknown` rather than guess the text leaf —
    /// asserting only the certainty (not-a-JWT), never simulating the semantic
    /// finding of *which* text type (that stays the model's job). Value-based
    /// (0048), demote-only, RHH-disableable.
    fn jwt_substance_guard(&self, result: &mut ColumnResult, sample: &[String]) {
        demote_when_substance_fails(
            result,
            sample,
            "technology.cryptographic.jwt",
            "jwt_substance_guard",
            finetype_core::structure::is_jwt,
        );
    }

    /// `mime_type_substance_guard` (default ON). Demotes `representation.file.mime_type`
    /// to `unknown` when the column's values are not real media types. The taxonomy
    /// pattern `^[a-zA-Z]+/[a-zA-Z0-9.+\-]+(;.*)?$` accepts ANY word as the
    /// top-level type, so the model over-emits `mime_type` on every `word/word`
    /// string at corpus scale (~1,372 live of the stale snapshot's 3,214 — slugs
    /// `recipes/…`, qualified paths `ccs/stc2010`/`geoId/15`, namespaces). A genuine
    /// media type leads with one of the ten RFC 6838 top-level types
    /// (`finetype_core::structure::is_mime_type`); when fewer than half a column's
    /// values carry that certainty the mime_type assertion is wrong. We demote to
    /// `unknown` rather than guess the text leaf — asserting only the certainty
    /// (not-a-media-type), never simulating *which* text type it is (that stays the
    /// model's job). Value-based (0048), demote-only, RHH-disableable. Twin of
    /// `jwt_substance_guard`; runs alongside it, after the recovery guards.
    fn mime_type_substance_guard(&self, result: &mut ColumnResult, sample: &[String]) {
        demote_when_substance_fails(
            result,
            sample,
            "representation.file.mime_type",
            "mime_type_substance_guard",
            finetype_core::structure::is_mime_type,
        );
    }

    /// `locale_code_substance_guard` (default ON). Demotes `technology.code.locale_code`
    /// to `unknown` when the column's values are not real locale codes. The taxonomy
    /// pattern `^[a-zA-Z]{2,3}(?:[-_][a-zA-Z]{2,4})*$` accepts ANY 2–3 letter word,
    /// so the model over-emits `locale_code` (largest tier-3 live surface, ~1,933 of
    /// a stale 4,246) on text/code columns — survey fragments, dialogue-act tags,
    /// single words. A genuine locale leads with a real ISO 639 language and any
    /// script/region subtag is a real ISO 15924 / 3166-1 code
    /// (`finetype_core::structure::is_locale_code`, delimiter-tolerant); when fewer
    /// than half a column's values carry that certainty the assertion is wrong. We
    /// demote to `unknown` — asserting only the certainty (not-a-locale), never
    /// guessing which text type it is (that stays the model's job). Value-based
    /// (0048), demote-only, RHH-disableable. Twin of `mime_type_substance_guard`;
    /// runs alongside it, after the recovery guards.
    ///
    /// The keep-bar stays at the ≥50% convention (not higher): the 2-letter ISO-639
    /// space is collision-dense, so a bare-2-letter-word column can pass the check —
    /// but under demote-only that is a harmless false-keep (status quo), never a
    /// false-demote. Calibration: `output/certainty-locale/findings.md`.
    fn locale_code_substance_guard(&self, result: &mut ColumnResult, sample: &[String]) {
        demote_when_substance_fails(
            result,
            sample,
            "technology.code.locale_code",
            "locale_code_substance_guard",
            finetype_core::structure::is_locale_code,
        );
    }

    /// `password_substance_guard` (default ON). Demotes `identity.person.password`
    /// to `unknown` when the column is plainly not a credential field. The taxonomy
    /// validator is `minLength: 1, maxLength: 255` (`designation: broad_characters`) —
    /// it certifies NOTHING, so the flat softmax scatters `password` onto free text
    /// at corpus scale: i18n/UI strings, song/anime/artist titles, country names,
    /// prose. On the corpus sample essentially NONE of the survivors are genuine
    /// passwords (real credential columns are PII and barely appear), so this is a
    /// near-pure false-positive attractor.
    ///
    /// Unlike mime/locale, a password has **no positive substance** — it is defined by
    /// the *absence* of structure (a high-entropy secret), so there is no "is-a-password"
    /// test to build (that would be simulating semantics, the certainty direction's one
    /// forbidden move). The guard therefore keys on the single self-precise **anti-signal**:
    /// a credential never contains internal whitespace, so a value with a space is not a
    /// password. When fewer than half the values are credential-shaped (whitespace-free),
    /// the column is text and the label is wrong → demote to `unknown` (asserting only the
    /// certainty *not-a-credential*, never guessing which text type). This catches the
    /// space-heavy bulk (phrases/titles/i18n); the whitespace-free residual (code
    /// identifiers) stays `password` — a harmless false-keep under demote-only, since the
    /// genuine-password population is ~empty. The rare passphrase-with-spaces column is the
    /// only theoretical false-demote, and such columns do not occur in practice.
    /// Value-based (0048), demote-only, RHH-disableable. Substrate: `output/certainty-password/`.
    fn password_substance_guard(&self, result: &mut ColumnResult, sample: &[String]) {
        // Credential-shaped = no internal whitespace. A password never contains a
        // space, so a value with one is not a password; when the majority carry
        // whitespace the column is text (phrases/titles/i18n) and the label is wrong.
        demote_when_substance_fails(
            result,
            sample,
            "identity.person.password",
            "password_substance_guard",
            |v| !v.chars().any(char::is_whitespace),
        );
    }

    /// `unlocode_format_veto` (default ON). UN/LOCODE is a closed 5-char shape
    /// (`^[A-Z]{2}[A-Z2-9]{3}$` — no space, no digit after the leading 2 letters).
    /// The model over-emits it at high confidence on value-identical short codes —
    /// notably alphanumeric postcodes (`CM13 3GF`, a UK postcode) that carry a
    /// space/digit unlocode forbids. unlocode is rare, so an assertion whose OWN
    /// values fail the unlocode pattern (`sample_contradicts_label`, <=10% pass) is
    /// a misfire. Demote-only. Shape-aware route: if the contradicting values pass a
    /// CONCRETE postal locale pattern, assert postal_code (lands the UK-postcode gold
    /// col); otherwise fall back to `unknown` — NOT the permissive universal postal
    /// block (minLength 3 / maxLength 10), per the Precision Principle. Value-based
    /// (0048); RHH-disableable.
    fn unlocode_format_veto(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("unlocode_format_veto")
            || result.label != "geography.transportation.unlocode"
        {
            return;
        }
        let Some(tax) = self.taxonomy.as_ref() else {
            return;
        };
        if !sample_contradicts_label(tax, "geography.transportation.unlocode", sample) {
            return;
        }
        // Route ONLY on a concrete locale match, NOT the permissive universal postal
        // block; otherwise demote to unknown.
        let locale = detect_locale_from_validation(sample, "geography.address.postal_code", tax);
        let target = if locale.is_some() {
            "geography.address.postal_code"
        } else {
            "unknown"
        };
        result.label = target.to_string();
        result.detected_locale = locale;
        result.confidence = result.confidence.min(0.6);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("unlocode_format_veto".to_string());
    }

    /// `legal_form_postal_demote` (default ON). Demotes a false
    /// `geography.address.postal_code` assertion off a legal-form code column
    /// (`legal_form`/`elf`) to `representation.identifier.numeric_code`. GLEIF's
    /// `legal_form` carries ISO 20275 Entity Legal Form codes, but 91% of the column
    /// is the reserved sentinels `9999` ("other") / `8888` ("no legal form"), which
    /// are 4-digit and trip the numeric-postal detector at confidence 1.0 (locale
    /// EN_PH). There is no ELF certainty worth building here — a column that is 91%
    /// "no legal form" would harvest 91% "N.A." from a membership set, and the 4-char
    /// shape is not self-precise — so this removes the false postal claim rather than
    /// manufacturing a hollow one. Target = `numeric_code` (author decision 2026-07-13,
    /// over the `word` categorical residual): the column is dominantly 4-digit
    /// classification codes, which an analyst reads as a numeric code, and its all-digit
    /// validator confirms the 91% majority (the ~9% alphabetic real ELF codes are the
    /// known imperfection — the trade is a concrete code label over the residual).
    /// Header-gated + demote-only: the `legal_form`/`elf` header is disjoint from every
    /// postal header (corpus-wide, 0 of 6 legal-form-headed columns are postal), and the
    /// `== postal_code` label gate means a genuine postal column — which never carries a
    /// legal-form header — is untouched. External-band finding (compref:gleif); the leaf
    /// is corpus-absent so the corpus-honest gate is structurally blind here — the header
    /// and demote-only shape is the safety, the same posture as every membership guard off
    /// the company-reference seam. Value-based (0048), RHH-disableable.
    fn legal_form_postal_demote(&self, result: &mut ColumnResult, header: &str) {
        if rhh::is_disabled("legal_form_postal_demote") {
            return;
        }
        if result.label != "geography.address.postal_code" || !header_names_legal_form(header) {
            return;
        }
        result.label = "representation.identifier.numeric_code".to_string();
        result.detected_locale = None;
        result.confidence = result.confidence.min(0.6);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!(
            "legal_form_postal_demote:{}",
            header.to_lowercase()
        ));
    }

    /// `isbn_header_recovery` (default ON). Recovers `identity.commerce.isbn` on a
    /// column whose header names ISBN and whose values carry a valid ISBN check
    /// digit, but which the 240-dim model funnelled into a digit-shaped lookalike
    /// (numeric_code / integer_number / npi / alphanumeric_id) — an ISBN is just
    /// digits, so neither the model nor `checksum_substance_guard` can tell it from
    /// a bare number. The mod-11 (ISBN-10) / mod-10 (ISBN-13) check digit is exactly
    /// what distinguishes a real ISBN column from a same-length integer, and the
    /// header confirms intent. Promote when the header matches AND >=90% of values
    /// pass `finetype_core::checksum::isbn`. Recovery-only: a non-ISBN numeric column
    /// fails the checksum and is untouched. Runs AFTER `checksum_substance_guard`
    /// (re-promotes from the integer that guard may have demoted to). Value-based
    /// (0048), RHH-disableable.
    fn isbn_header_recovery(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("isbn_header_recovery") {
            return;
        }
        const LEAF: &str = "identity.commerce.isbn";
        if result.label == LEAF || !header_corroborates_isbn(header) {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        let valid = non_empty
            .iter()
            .filter(|v| finetype_core::checksum::isbn(v))
            .count();
        // >=90% pass the ISBN check digit (valid*10 >= len*9).
        if valid * 10 < non_empty.len() * 9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.95);
        result.disambiguation_applied = true;
        result.disambiguation_rule =
            Some(format!("isbn_header_recovery:{}", header.to_lowercase()));
        result.detected_locale = None;
    }

    /// `numeric_code_header_recovery` (default ON). Feature rule F5 demotes
    /// `numeric_code` to `integer_number` whenever a column has no leading
    /// zeros — which structurally locks every no-leading-zero code system
    /// (NAICS sectors run 11–92, EDGAR CIKs never start with 0) out of its
    /// intended type: the model predicts numeric_code and the deterministic
    /// layer erases it (company-reference audit, gold rows compref:naics /
    /// compref:sec_edgar). Values alone cannot separate "digits that identify"
    /// from "digits that quantify", so this is a 0094 header-corroboration
    /// boundary: restore numeric_code when the header names a code column
    /// (`header_corroborates_numeric_code` — token-aware, postal tokens veto)
    /// AND ≥90% of values are all-digit with median length ≥2 (bare 0/1 flag
    /// columns stay integers). Recovery-only: quantity columns (`employees`,
    /// `founded`) carry no code token and are untouched. Value-based veto
    /// discipline + header corroboration per 0094; RHH-disableable.
    ///
    /// Also gates on an IDENTIFIER header (`header_names_numeric_identifier`:
    /// `id`/`ids`/`identifier`) — `*_id` integer columns are the single most common
    /// mistake on a random production column (representative band 2026-07-13:
    /// `PLAYER_ID`/`GAME_ID`/`student_id` typed as quantities to average). An id is
    /// never a quantity, so it recovers off the integer attractor — and it SPLITS by
    /// the model's own `values_form_increment`: a running surrogate key (dense,
    /// contiguous, near-unique) → `increment`; an opaque id → `numeric_code`. The
    /// split matches the author-ratified numeric-id gold/repr reconciliation
    /// (2026-07-13) — gold's ac-04 panel had lumped numeric ids into integer_number;
    /// the representative band already used the split. The id gate is deliberately a
    /// SEPARATE function from the code gate so it does NOT widen the naics gate (which
    /// shares `header_corroborates_numeric_code`) — an id column of arbitrary integers
    /// must never leak into naics.
    fn numeric_code_header_recovery(
        &self,
        result: &mut ColumnResult,
        header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("numeric_code_header_recovery") {
            return;
        }
        let id_header = header_names_numeric_identifier(header);
        if result.label != "representation.numeric.integer_number"
            || !(header_corroborates_numeric_code(header) || id_header)
        {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        let all_digit = non_empty
            .iter()
            .filter(|v| !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()))
            .count();
        if all_digit * 10 < non_empty.len() * 9 {
            return;
        }
        let mut lens: Vec<usize> = non_empty.iter().map(|v| v.len()).collect();
        lens.sort_unstable();
        if lens[lens.len() / 2] < 2 {
            return;
        }
        // Split an identifier column by shape: a running surrogate key is `increment`,
        // an opaque id (and any code-headed column) is `numeric_code`.
        let leaf = if id_header && values_form_increment(sample) == Some(true) {
            "representation.identifier.increment"
        } else {
            "representation.identifier.numeric_code"
        };
        result.label = leaf.to_string();
        result.confidence = result.confidence.max(0.85);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!(
            "numeric_code_header_recovery:{}",
            header.to_lowercase()
        ));
        result.detected_locale = None;
    }

    /// `ceded_leaf_recovery` (default ON). The recall side of the model label-space
    /// reshape (spec 2026-06-27-model-label-space-reshape ac-3). The reshaped Sense
    /// model no longer emits the closed/format/checksum leaves (they were dropped
    /// from its softmax head); this re-asserts them deterministically from VALUES,
    /// so recall is recovered, not lost.
    ///
    /// Fires when ≥90% of the sample passes exactly ONE eligible leaf's OWN strict
    /// taxonomy validator (`label_validates_sample` — the same gate the validation
    /// veto uses, so the two stay a single source of truth). The eligible set is the
    /// ac-0 CONCLUSIVE (value-self-sufficient) cede subset MINUS the leaves a
    /// more-specific rule already owns (datetime FORMATS → `datetime_format_refinement`;
    /// url/message_id/windows_path/qualified_name → `structured_string_refinement`;
    /// isbn → `isbn_header_recovery`) MINUS the permissive-validator leaves the ac-0
    /// adversarial challenge demoted (color_hex/rgb — optional `#` lets bare numbers
    /// pass). Each remaining validator is structurally exclusive, so a match is
    /// correct-by-construction.
    ///
    /// AUTHORITATIVE OVERRIDE: a conclusive validator may override even a confident
    /// model prediction (a real `uuid` IS a uuid, not the `npi`/`tsid` the reshaped
    /// model relocates it to — ac-2 destination drift). EXACTLY-ONE-MATCH gate: never
    /// guess on overlap; if two eligible leaves both validate, defer (the common
    /// overlaps — ip_v4 vs ip_v4_with_port, json vs json_array — are mutually
    /// exclusive in practice, so this rarely costs recall). Value-based (decision
    /// 0048); RHH-disableable.
    fn ceded_leaf_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("ceded_leaf_recovery") {
            return;
        }
        let Some(tax) = self.taxonomy.as_ref() else {
            return;
        };
        let non_empty = sample.iter().filter(|v| !v.trim().is_empty()).count();
        if non_empty < 3 {
            // Too few values to assert a conclusive type safely.
            return;
        }
        let mut matches = CEDED_RECOVERY_LEAVES
            .iter()
            .copied()
            .filter(|leaf| *leaf != result.label.as_str())
            .filter(|leaf| label_validates_sample(tax, leaf, sample));
        let Some(leaf) = matches.next() else {
            return;
        };
        // Ambiguity guard: a second eligible leaf also validates — do not guess.
        if matches.next().is_some() {
            return;
        }
        result.label = leaf.to_string();
        result.confidence = result.confidence.max(0.9);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("ceded_leaf_recovery".to_string());
        result.detected_locale = detect_locale_from_validation(sample, leaf, tax);
    }

    /// `isin_checksum_recovery` (default ON). Corrects the one non-exclusive
    /// overlap in the ceded set: `identity.commerce.isrc` (12 chars, shape-only)
    /// and `finance.securities.isin` (12 chars + ISIN check digit) share a shape,
    /// and a digit-tailed ISIN matches ISRC's `^[A-Z]{2}[A-Z0-9]{3}\d{7}$` pattern.
    /// `validate_value_for_label` is regex-only, so `ceded_leaf_recovery` cannot
    /// see the check digit and mislabels a real ISIN column as isrc. Promote to
    /// isin when the sample matches the ISIN shape (2-letter country prefix,
    /// excludes bare 12-digit codes) AND ≥90% pass the ISIN check digit
    /// (`finetype_core::checksum::isin`) — the discriminator isrc cannot satisfy.
    /// Runs after `ceded_leaf_recovery` so it has the last word over the isrc
    /// misassertion. Value-based (decision 0048); RHH-disableable.
    fn isin_checksum_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("isin_checksum_recovery") {
            return;
        }
        if result.label == "finance.securities.isin" {
            return;
        }
        let Some(tax) = self.taxonomy.as_ref() else {
            return;
        };
        // Shape gate: the ISIN pattern requires a 2-letter country prefix, which
        // excludes bare 12-digit codes that could coincidentally pass the check.
        if !label_validates_sample(tax, "finance.securities.isin", sample) {
            return;
        }
        let mut checked = 0usize;
        let mut passed = 0usize;
        for v in sample {
            let t = v.trim();
            if t.is_empty() {
                continue;
            }
            checked += 1;
            if finetype_core::checksum::isin(t) {
                passed += 1;
            }
        }
        if checked < 3 || (passed as f64) / (checked as f64) < 0.9 {
            return;
        }
        result.label = "finance.securities.isin".to_string();
        result.confidence = result.confidence.max(0.9);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("isin_checksum_recovery".to_string());
    }

    /// `cusip_checksum_recovery` (default ON). The 244-dim model cannot predict
    /// `finance.securities.cusip` (a 9-char security id: 8 issuer/issue chars + a
    /// check digit), so real CUSIP columns land on the word / alphanumeric_id /
    /// integer_number attractor. The taxonomy shape `^[A-Z0-9]{8}[0-9]$` also
    /// matches any 9-char alnum id (SKUs, 9-digit account numbers), so shape alone
    /// is not enough. The CUSIP mod-10 check digit (`finetype_core::checksum::cusip`)
    /// is the discriminator the attractor cannot satisfy: a non-CUSIP id's 9th char
    /// passes that arithmetic only ~1 in 10 by chance, so a >=90% column-wide pass is
    /// unreachable for anything but a genuine CUSIP column. Value-only — no header,
    /// round-trips headerless. ISIN is 12 chars and SEDOL 7, so nothing else in the
    /// finance family competes at this length. Value-based (0048); RHH-disableable.
    fn cusip_checksum_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("cusip_checksum_recovery") {
            return;
        }
        const LEAF: &str = "finance.securities.cusip";
        if result.label == LEAF {
            return;
        }
        let Some(tax) = self.taxonomy.as_ref() else {
            return;
        };
        // Shape gate: `^[A-Z0-9]{8}[0-9]$`. Mirrors isin_checksum_recovery's use of
        // the leaf's own taxonomy validator before the check-digit test.
        if !label_validates_sample(tax, LEAF, sample) {
            return;
        }
        let mut checked = 0usize;
        let mut passed = 0usize;
        let mut distinct_pass: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for v in sample {
            let t = v.trim();
            if t.is_empty() {
                continue;
            }
            checked += 1;
            if finetype_core::checksum::cusip(t) {
                passed += 1;
                distinct_pass.insert(t);
            }
        }
        // >=3 DISTINCT passing values, not just >=90% of a repeated one: a constant /
        // low-cardinality numeric column (gold `phkey` = one repeated 9-digit key
        // 484158167) can pass the weak mod-10 CUSIP check at 100% by coincidence, so a
        // single coincidental pass must not assert this rare type — require the column
        // to carry distributional evidence.
        if checked < 3 || distinct_pass.len() < 3 || (passed as f64) / (checked as f64) < 0.9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.9);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("cusip_checksum_recovery".to_string());
    }

    /// `sedol_checksum_recovery` (default ON). Recovers `finance.securities.sedol`.
    /// The 244-dim model cannot predict this leaf, so a real SEDOL column lands on
    /// its numeric_code / alphanumeric_id attractor. The SEDOL shape (6 no-vowel
    /// alphanumerics + a trailing check digit) is shared by bare 7-digit numeric
    /// codes, so SHAPE alone cannot recover it — the check digit is the discriminator
    /// a numeric code cannot satisfy. Promote when the sample passes the sedol
    /// taxonomy shape (uppercase, no vowels, 7 chars — constraints the checksum fn's
    /// alnum_value does NOT itself impose) AND >=90% pass the SEDOL check digit
    /// (`finetype_core::checksum::sedol`). Value-only, like isin_checksum_recovery:
    /// a random 7-char column passes the 1/10 check ~10% of the time, far below the
    /// 90% bar, so it round-trips headerless. Value-based (0048); RHH-disableable.
    fn sedol_checksum_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("sedol_checksum_recovery") {
            return;
        }
        const LEAF: &str = "finance.securities.sedol";
        if result.label == LEAF {
            return;
        }
        let Some(tax) = self.taxonomy.as_ref() else {
            return;
        };
        if !label_validates_sample(tax, LEAF, sample) {
            return;
        }
        let mut checked = 0usize;
        let mut passed = 0usize;
        let mut distinct_pass: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for v in sample {
            let t = v.trim();
            if t.is_empty() {
                continue;
            }
            checked += 1;
            if finetype_core::checksum::sedol(t) {
                passed += 1;
                distinct_pass.insert(t);
            }
        }
        // >=3 DISTINCT passing values: same constant-column guard as cusip — a bare
        // 7-digit numeric code repeated across a low-cardinality column can pass the
        // weighted SEDOL check by coincidence, so require distributional evidence.
        if checked < 3 || distinct_pass.len() < 3 || (passed as f64) / (checked as f64) < 0.9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.9);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("sedol_checksum_recovery".to_string());
    }

    /// `dea_checksum_recovery` (default ON). The 244-dim model cannot predict
    /// `identity.medical.dea_number`; a real DEA column lands on the `alphanumeric_id`
    /// attractor (2 letters + 7 digits reads as a generic letter+digit id). Promote to
    /// dea_number when the sample matches the DEA shape
    /// (`^[ABFMPRabfmpr][A-Za-z]\d{7}$`, which pins the first char to a registrant-type
    /// letter the checksum fn does NOT check) AND >=90% pass the DEA check digit
    /// (`finetype_core::checksum::dea`). The mod-10 formula is DEA-specific — no
    /// non-target id scheme (credit cards use Luhn) is built to satisfy it, so a
    /// same-shape alphanumeric_id column passes at ~10%, well under the bar. Self-precise
    /// value signal → no header gate; round-trips headerless. Value-based (0048),
    /// RHH-disableable.
    fn dea_checksum_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("dea_checksum_recovery") {
            return;
        }
        const LEAF: &str = "identity.medical.dea_number";
        if result.label == LEAF {
            return;
        }
        let Some(tax) = self.taxonomy.as_ref() else {
            return;
        };
        // Shape gate: the dea_number pattern pins the first char to a registrant-type
        // letter {A,B,F,M,P,R}, which checksum::dea alone does not enforce.
        if !label_validates_sample(tax, LEAF, sample) {
            return;
        }
        let mut checked = 0usize;
        let mut passed = 0usize;
        let mut distinct_pass: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for v in sample {
            let t = v.trim();
            if t.is_empty() {
                continue;
            }
            checked += 1;
            if finetype_core::checksum::dea(t) {
                passed += 1;
                distinct_pass.insert(t);
            }
        }
        // >=3 DISTINCT passing values: same constant-column guard as cusip/sedol. The
        // DEA shape needs two leading letters (an all-numeric column can never match),
        // so the risk is lower, but the cardinality bar keeps the three uniform.
        if checked < 3 || distinct_pass.len() < 3 || (passed as f64) / (checked as f64) < 0.9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.9);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("dea_checksum_recovery".to_string());
    }

    /// `imei_checksum_recovery` (default ON). Recovers `technology.code.imei`. The
    /// 244-dim model cannot predict it, so a genuine 15-digit IMEI column lands on
    /// `integer_number`. Luhn ALONE is NOT a discriminator here — a 15-digit American
    /// Express card column is Luhn-valid BY CONSTRUCTION — so promote requires an
    /// `imei` header (`header_corroborates_imei`, which a payment header never yields)
    /// AND a value gate: >=90% exactly-15-digit Luhn-valid values. The two gates are
    /// jointly load-bearing — the header excludes the Amex collision the value gate
    /// cannot. Value-based (0048); RHH-disableable.
    fn imei_checksum_recovery(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("imei_checksum_recovery") {
            return;
        }
        const LEAF: &str = "technology.code.imei";
        if result.label == LEAF || !header_corroborates_imei(header) {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        let valid = non_empty
            .iter()
            .filter(|v| {
                v.len() == 15
                    && v.bytes().all(|b| b.is_ascii_digit())
                    && finetype_core::checksum::luhn(v)
            })
            .count();
        // >=90% are exactly-15-digit Luhn-valid IMEIs (valid*10 >= len*9).
        if valid * 10 < non_empty.len() * 9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.95);
        result.disambiguation_applied = true;
        result.disambiguation_rule =
            Some(format!("imei_checksum_recovery:{}", header.to_lowercase()));
        result.detected_locale = None;
    }

    /// `cpt_procedure_recovery` (default ON). Recovers `identity.medical.cpt`. The
    /// 244-dim model cannot predict this leaf; the bare 5-digit form lands on the
    /// numeric_code / integer / postal attractor. CPT has no check digit or membership
    /// set, and `^\d{5}$` is value-identical with a US ZIP code, so the distinctive
    /// `cpt`/`procedure` header token (`header_corroborates_cpt`, NO generic `code`
    /// tier — that would admit a ZIP column headed `code`) is the SOLE discriminator.
    /// Promote when the header corroborates AND >=90% pass the CPT taxonomy validator
    /// (`^\d{5}$|^\d{4}[FTU]$`). Value-based (0048); RHH-disableable.
    fn cpt_procedure_recovery(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("cpt_procedure_recovery") {
            return;
        }
        const LEAF: &str = "identity.medical.cpt";
        if result.label == LEAF || !header_corroborates_cpt(header) {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        match self.taxonomy.as_ref() {
            Some(tax) if label_validates_sample(tax, LEAF, sample) => {
                result.label = LEAF.to_string();
                result.confidence = result.confidence.max(0.95);
                result.disambiguation_applied = true;
                result.disambiguation_rule =
                    Some(format!("cpt_procedure_recovery:{}", header.to_lowercase()));
                result.detected_locale = None;
            }
            _ => {}
        }
    }

    /// `hs_code_header_recovery` (default ON). Recovers the
    /// `geography.transportation.hs_code` leaf. The 244-dim model cannot predict it, so
    /// a customs-tariff column lands on the numeric / `word` attractor (the multi-dot
    /// forms like `0901.11.00.10` are not valid decimals, so the model reads `word`);
    /// R20 `hs_code_validation_gate` in value_sharpen only DEMOTES a wrong hs_code, it
    /// never recovers one the model never emitted. Header-gated because HS codes carry
    /// NO check digit and the bare form is value-identical to a plain integer/year: the
    /// header token (`header_corroborates_hs_code`) is the sole discriminator. A
    /// median-length floor of 6 kills the bare-4-digit-year false pass that
    /// `is_hs_code_format`'s loose no-dot branch admits. Promote when the header
    /// corroborates AND median length >=6 AND >=90% pass `is_hs_code_format`.
    /// Value-based (0048); RHH-disableable.
    fn hs_code_header_recovery(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("hs_code_header_recovery") {
            return;
        }
        const LEAF: &str = "geography.transportation.hs_code";
        if result.label == LEAF || !header_corroborates_hs_code(header) {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        // Length floor: is_hs_code_format's no-dot branch accepts bare 4-5 digit
        // numbers (years, small ints) the taxonomy pattern (min 6 digits) rejects;
        // require median length >= 6 so a 4-digit-year column cannot pass.
        let mut lens: Vec<usize> = non_empty.iter().map(|v| v.len()).collect();
        lens.sort_unstable();
        if lens[lens.len() / 2] < 6 {
            return;
        }
        let valid = non_empty.iter().filter(|v| is_hs_code_format(v)).count();
        // >=90% match the HS digit-group format (valid*10 >= len*9).
        if valid * 10 < non_empty.len() * 9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.95);
        result.disambiguation_applied = true;
        result.disambiguation_rule =
            Some(format!("hs_code_header_recovery:{}", header.to_lowercase()));
        result.detected_locale = None;
    }

    /// `unlocode_membership_recovery` (default ON). Promotes the
    /// `geography.transportation.unlocode` leaf (the twin of the demote-only
    /// `unlocode_format_veto`). The 244-dim model cannot predict it, so a real UN/LOCODE
    /// column (`USLAX`/`GBLON`) lands on the `word` attractor. The 5-char `CC + 3`
    /// shape is shared by stock tickers and SKUs, so SHAPE alone cannot recover it —
    /// EXACT membership in the published UN/LOCODE set (`membership::unlocode`, ~0.5% of
    /// the shape space AND each member needs a valid ISO-3166-1 country prefix) is the
    /// discriminator a ticker/SKU column is ~never 90% inside. NO header gate — membership
    /// at 0.90 density is self-precise, so it round-trips headerless. The demote-only
    /// veto owns the label==unlocode case, so this promote and that veto never touch the
    /// same column. Value-based (0048); RHH-disableable.
    fn unlocode_membership_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("unlocode_membership_recovery") {
            return;
        }
        const LEAF: &str = "geography.transportation.unlocode";
        if result.label == LEAF {
            return;
        }
        let mut checked = 0usize;
        let mut passed = 0usize;
        let mut distinct_pass: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for v in sample {
            let t = v.trim();
            if t.is_empty() {
                continue;
            }
            checked += 1;
            if finetype_core::membership::unlocode(t) {
                passed += 1;
                distinct_pass.insert(t);
            }
        }
        // >=90% published UN/LOCODE membership AND >=3 DISTINCT passing values. The
        // distinct gate is load-bearing: the 110k-entry set makes 5-char collisions
        // common, so a constant / low-cardinality column (a fund `symbol` column of one
        // repeated ticker `FRMUF`/`DELRF`, a `city` column of `Essen`, a `Namespace`
        // column of `Debug`) matches a single UN/LOCODE entry at 100% by coincidence.
        // A single coincidental match carries no distributional evidence and must not
        // assert this type — mirror cusip/sedol/dea_checksum_recovery's distinct gate.
        if checked < 3 || distinct_pass.len() < 3 || passed * 10 < checked * 9 {
            return;
        }
        let from = result.label.clone();
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.85);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("unlocode_membership_recovery:{from}"));
        result.detected_locale = None;
    }

    /// `color_rgb_recovery` (default ON). Recovers `representation.format.color_rgb`
    /// (not in the 244-dim softmax) from VALUES. The taxonomy validator
    /// `^(?:rgb)?\(?(\d{1,3}),\s*(\d{1,3}),\s*(\d{1,3})\)?$` is PERMISSIVE — `rgb`,
    /// `(` and `)` are all optional, so a bare comma triple `255,0,0` (a coordinate, a
    /// comma_separated array, or the `word` attractor) passes; that is why color_rgb was
    /// excluded from `ceded_leaf_recovery`. This PROMOTE guard restores recall precisely
    /// by requiring the literal `rgb(` / `rgba(` prefix a bare triple cannot carry —
    /// self-precise, so NO header gate (mirror of `s_expression_recovery`). Value-based
    /// (0048); RHH-disableable.
    fn color_rgb_recovery(&self, result: &mut ColumnResult, sample: &[String]) {
        if rhh::is_disabled("color_rgb_recovery") {
            return;
        }
        const LEAF: &str = "representation.format.color_rgb";
        if result.label == LEAF {
            return;
        }
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        // Anchored substance check: literal rgb(/rgba( prefix + closing ) + 3 (rgb)
        // or 4 (rgba) comma components whose first three are integers in 0..=255.
        let is_rgb = |raw: &str| -> bool {
            let lower = raw.trim().to_ascii_lowercase();
            let inner = lower
                .strip_prefix("rgba(")
                .or_else(|| lower.strip_prefix("rgb("));
            let Some(inner) = inner.and_then(|s| s.strip_suffix(')')) else {
                return false;
            };
            let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
            if parts.len() != 3 && parts.len() != 4 {
                return false;
            }
            parts.iter().take(3).all(|p| {
                let n = p.strip_suffix('%').unwrap_or(p);
                n.parse::<u16>().map(|v| v <= 255).unwrap_or(false)
            })
        };
        let valid = non_empty.iter().filter(|v| is_rgb(v)).count();
        // >=90% carry the literal rgb(...) certainty.
        if valid * 10 < non_empty.len() * 9 {
            return;
        }
        result.label = LEAF.to_string();
        result.confidence = result.confidence.max(0.95);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("color_rgb_recovery".to_string());
        result.detected_locale = None;
    }

    /// `increment_substance_veto` (default ON). The first full-column-statistics
    /// rule (spec 2026-06-16-column-statistics-lever). A genuine auto-increment
    /// (`representation.identifier.increment`) is a CONTIGUOUS, near-unique run
    /// of non-negative integers — a fact only the full column reveals. The
    /// `value_sharpen` sequential check runs on the 100-value STEPPED sample,
    /// where a true 1..N run becomes `1, k, 2k, …` (uniform diffs but not
    /// contiguous) and any evenly-spaced numeric column looks sequential — so it
    /// over-emits increment badly (gold precision 0.056: 1 TP, 17 FP, all gold
    /// `integer_number`). Re-check the FULL column: a real increment fills its
    /// own range (distinct ≈ max−min+1) with almost no duplicates. Otherwise the
    /// values are a plain integer column wearing the wrong label — demote to
    /// `integer_number`. Value-based (decision 0048); demotion-only, so a true
    /// increment passes through untouched.
    fn increment_substance_veto(&self, result: &mut ColumnResult, values: &[String]) {
        if rhh::is_disabled("increment_substance_veto")
            || result.label != "representation.identifier.increment"
        {
            return;
        }
        // Keep `increment` whenever the FULL column is a confirmed contiguous near-unique
        // run (`Some(true)`); demote `None` (too few integers to judge) and `Some(false)`
        // (a stepped sample that only looks sequential) to integer. NOTE: the aggressive
        // "default everything to integer unless a counter header" variant was corpus-honest
        // NO-GO — the gated-YDF oracle confirms ~128k contiguous-run increments on the
        // corpus, so wholesale demotion collapses 98% of its oracle support. The
        // increment-vs-integer-ID boundary is corpus-contested; gold's 8 integer-ID
        // over-emits cannot be recovered without that collapse, so only the value-decidable
        // None/Some(false) slice is taken (spec 2026-06-27-composed-accuracy-roadmap #12).
        if values_form_increment(values) == Some(true) {
            return;
        }
        result.label = "representation.numeric.integer_number".to_string();
        result.confidence = result.confidence.min(0.6);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("increment_substance_veto".to_string());
        if let Some(taxonomy) = self.taxonomy.as_ref() {
            result.detected_locale = detect_locale_from_validation(values, &result.label, taxonomy);
        }
    }

    /// `binary_vocab_veto` (default ON). `representation.boolean.binary` requires
    /// a strictly two-valued {0,1} domain, but the model over-emits it on sparse
    /// integer COUNT columns — mostly zeros with the occasional larger value
    /// (`Points` …0,0,8,0…; `Comments` …5,0,0…). Acts only on all-integer columns
    /// (so true/false/yes/no binaries are untouched): if any value falls outside
    /// {0,1}, the column is a count, not a flag — demote to integer. Genuine
    /// numeric binaries ({0,1} only) carry no out-of-domain value and pass
    /// through. Measured on gold: 12 of 18 integer columns the model called
    /// `binary` carry a value above 1.
    ///
    /// Scans the FULL column (`values`), not the down-sampled `sample` (BACKLOG
    /// #14): the rare value above 1 that proves "count, not flag" is exactly what
    /// stride-sampling drops (`Comments` … 5/14/30 in 3 of 162 rows; `noc` … 17;
    /// `confirmed` … 81397), so a sample-only scan misses it and leaves the count
    /// mislabelled `binary`.
    fn binary_vocab_veto(
        &self,
        result: &mut ColumnResult,
        header: &str,
        sample: &[String],
        values: &[String],
    ) {
        if rhh::is_disabled("binary_vocab_veto") || result.label != "representation.boolean.binary"
        {
            return;
        }
        let non_empty = non_empty_trimmed(values);
        if non_empty.len() < 3 {
            return;
        }
        let mut any_outside_01 = false;
        for v in &non_empty {
            match v.parse::<i64>() {
                Ok(n) => {
                    if n != 0 && n != 1 {
                        any_outside_01 = true;
                    }
                }
                // Non-integer value (true/false, decimal, text) — not a numeric
                // binary this veto should touch.
                Err(_) => return,
            }
        }
        if !any_outside_01 {
            return;
        }
        result.label = "representation.numeric.integer_number".to_string();
        result.confidence = result.confidence.min(0.6);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("binary_vocab_veto:{}", header.to_lowercase()));
        if let Some(taxonomy) = self.taxonomy.as_ref() {
            result.detected_locale = detect_locale_from_validation(sample, &result.label, taxonomy);
        }
    }

    /// `checksum_substance_guard` (default ON). Generic substance check for the
    /// self-validating identifier types — replaces the per-type
    /// `*_checkdigit_veto` pattern. Any column the model labels with a
    /// checksum-bearing type (the taxonomy `checksum:` directive — isbn, aba,
    /// cusip, sedol today) is re-checked against the real check-digit arithmetic
    /// from the canonical `finetype_core::checksum` module rather than a
    /// hand-rolled copy.
    ///
    /// The taxonomy's shape pattern (10 or 13 digits) lets a large financial
    /// integer like `marketCap` 5150000128 look like an ISBN; the checksum is
    /// what distinguishes "is this type" from "is not". A column whose values
    /// mostly FAIL their type's checksum is not that type — demote by value
    /// shape: bare-number columns (ISBN/ABA lookalikes) to decimal/integer,
    /// alphanumeric columns (CUSIP/SEDOL lookalikes) to alphanumeric_id or
    /// categorical by cardinality. Genuine identifiers pass the checksum and are
    /// untouched. Measured on gold: marketCap/otherLiab/… emitted as `isbn`,
    /// longTermInvestments/… as `aba_routing`, citation_id/case_number as
    /// `cusip`/`sedol`.
    ///
    /// The checksum is owned HERE, not in the shared compiled validator. Wiring
    /// it into the validator looks tidier but regresses: the generic
    /// `value_sharpen` schema-demotion rules also consult that validator and,
    /// on a checksum-failing column, demote it to their own fallback
    /// (`numeric_code`/categorical) — a worse target than the gold-correct
    /// `integer_number` this guard produces, and they run first. So the
    /// directive scopes this guard while the validator stays shape-only.
    fn checksum_substance_guard(
        &self,
        result: &mut ColumnResult,
        _header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("checksum_substance_guard") {
            return;
        }
        let Some(taxonomy) = self.taxonomy.as_ref() else {
            return;
        };
        // Only act on labels carrying a `checksum:` directive; resolve its
        // canonical validating function from crate::checksum.
        let Some(checksum) = taxonomy
            .get(&result.label)
            .and_then(|def| def.checksum.as_deref())
            .and_then(finetype_core::checksum::resolve)
        else {
            return;
        };
        demote_by_shape_when_substance_fails(
            result,
            sample,
            taxonomy,
            "checksum_substance_guard",
            checksum,
        );
    }

    /// `membership_substance_guard` (default ON). Twin of
    /// `checksum_substance_guard` for types whose substance is CLOSED-SET
    /// membership rather than a check digit (the taxonomy `membership:`
    /// directive — icao_airports, iata_airports, naics_codes, tld, unlocode
    /// today; see `finetype_core::membership` and labels/sets/). The taxonomy's shape
    /// pattern (`^[A-Z]{4}$` / `^[A-Z]{3}$`) confirms every same-shape token —
    /// a 4-letter stock-ticker column validates 100% as icao_code, which not
    /// only survives the veto but disarms the attractor demotion
    /// (`validation_confirmed`). Membership is what distinguishes "is this
    /// type" from "is not": a column whose values are mostly OUTSIDE the
    /// published code list is not that type — demote by value shape exactly as
    /// the checksum guard does. Genuine airport-code columns are list members
    /// and untouched. Company-reference audit W2
    /// (output/company-reference-audit/findings_and_action_plan.md).
    fn membership_substance_guard(
        &self,
        result: &mut ColumnResult,
        _header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("membership_substance_guard") {
            return;
        }
        let Some(taxonomy) = self.taxonomy.as_ref() else {
            return;
        };
        // Only act on labels carrying a `membership:` directive; resolve its
        // canonical set from crate::membership.
        let Some(is_member) = taxonomy
            .get(&result.label)
            .and_then(|def| def.membership.as_deref())
            .and_then(finetype_core::membership::resolve)
        else {
            return;
        };
        demote_by_shape_when_substance_fails(
            result,
            sample,
            taxonomy,
            "membership_substance_guard",
            is_member,
        );
    }

    /// `url_bare_number_veto` (default ON). Twin of `amount_bare_number_veto`.
    /// A link-ish header (`Publisher URL`, `perm_unlink`, `link_type`) makes the
    /// header-hint machinery promote the column to `technology.internet.url` via
    /// an early `return`, so the early `schema_validation_gate` never re-checks
    /// the hint's assertion. But a URL is text (`https://…`); a column of bare
    /// numbers (`0`/`1`/`-1` flags, large integers) cannot be one. Undo the
    /// hint's promotion by value shape: a bare-number `url` is demoted to
    /// decimal/integer. Demotion only — genuine URL columns contain non-numeric
    /// values and pass through untouched. Measured on gold: 6 whole-number
    /// columns headed `Publisher URL`/`perm_unlink`/`link_type` were emitted as
    /// `url` (url precision 0.721).
    fn url_bare_number_veto(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("url_bare_number_veto") || result.label != "technology.internet.url" {
            return;
        }
        let (is_bare, any_decimal) = values_look_like_bare_numbers(sample);
        if is_bare {
            demote_bare_to_numeric(
                result,
                sample,
                self.taxonomy.as_ref(),
                any_decimal,
                "url_bare_number_veto",
                header,
            );
        }
    }

    /// `utc_bare_number_veto` (default ON). Twin of `url_bare_number_veto`.
    /// A `utc_offset`/`tz`-ish header makes the header-hint machinery promote the
    /// column to `datetime.offset.utc` via an early `return`, so the early
    /// `schema_validation_gate` never re-checks the hint's assertion. But the
    /// taxonomy's `datetime.offset.utc` is an explicit offset STRING ("UTC
    /// +05:00", validator `^UTC [+-]\d{2}:\d{2}$`); a column of bare hour-offsets
    /// (`-8`, `5.5`, `0`, `3.5`) cannot be one — it fails that validator ~100%.
    /// Undo the hint's promotion by value shape: a bare-number `utc` is demoted
    /// to decimal/integer. Demotion only — a genuine `UTC +HH:MM` column contains
    /// non-numeric values and passes through untouched. Measured on gold: 5
    /// `utc_offset` columns (OpenFlights + 4 gittables) were emitted as
    /// `datetime.offset.utc`; four are already gold-labelled decimal (the fifth
    /// re-adjudicated to decimal — same bare-number shape). Value-based last-resort
    /// Sharpen (decisions 0038/0048); the v24 CORROBORATION_SCOPE add for utc was
    /// inert/regressive, so this is the correct lever (roadmap #3).
    fn utc_bare_number_veto(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("utc_bare_number_veto") || result.label != "datetime.offset.utc" {
            return;
        }
        let (is_bare, _) = values_look_like_bare_numbers(sample);
        if is_bare {
            // A numeric UTC-offset column is decimal the moment it carries a
            // half/quarter-hour zone (+5.5, -3.5, +5.45). Those sort after the
            // integer offsets, beyond the 16-value window
            // `values_look_like_bare_numbers` scans, so decide decimal-vs-integer
            // over the WHOLE sample here — otherwise an integer-headed column
            // (OpenFlights `utc_offset`) misses its true decimal type.
            let any_decimal = sample
                .iter()
                .any(|v| v.contains('.') && v.trim().parse::<f64>().is_ok());
            demote_bare_to_numeric(
                result,
                sample,
                self.taxonomy.as_ref(),
                any_decimal,
                "utc_bare_number_veto",
                header,
            );
        }
    }

    /// `city_region_header_corroboration` (default ON). The model's
    /// `geography.location.city` and `.region` are value-identical siblings —
    /// both are textual place names — so the flat softmax confuses them. When the
    /// header explicitly names an administrative division above city level
    /// (`region`, `county`, `district`, `borough`, `province`, …) the header is
    /// the disambiguator: promote a `city` prediction to `region`. Measured on
    /// gold: 6 `region` columns headed `Region`/`County`/`district`/`borough`
    /// (Zimbabwe & Ecuador provinces, US counties) were emitted as `city`
    /// (region recall 0.467, city precision 0.632). Header-gated and
    /// promotion-only, so genuine cities — which do not carry admin-division
    /// headers — are untouched. Follows the choice-0094 header-corroboration
    /// pattern; ships default-on pending the corpus-honest relocation gate.
    fn city_region_header_corroboration(
        &self,
        result: &mut ColumnResult,
        header: &str,
        _sample: &[String],
    ) {
        if rhh::is_disabled("city_region_header_corroboration")
            || result.label != "geography.location.city"
            || !header_corroborates_region(header)
        {
            return;
        }
        result.label = "geography.location.region".to_string();
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!(
            "city_region_header_corroboration:{}",
            header.to_lowercase()
        ));
    }

    /// `country_code_corroboration` (default ON). Two-letter ISO 3166-1 codes
    /// (`US`, `HK`, `MT`) are value-identical to other short geographic tokens,
    /// so the flat softmax files them under `region`/`state`/`city`/`country`.
    /// The `country_code` enum — 249 official ISO codes, the taxonomy's most
    /// precise validator — is the disambiguator: a confusable geography label
    /// whose values are MOSTLY valid ISO codes is a country_code. Value-based
    /// (decision 0048) and promotion-only — genuine region/state/city columns
    /// carry place names, not ISO codes, so they fail the enum and are
    /// untouched. `state`/`state_code` are deliberately NOT confusable here:
    /// their 2-letter codes legitimately overlap the ISO set, so promoting them
    /// would trade one error for another. Measured on gold: 5 columns
    /// (country_code_filter=MT, exchange_country=HK, PA, id=AU/AT/…, Country)
    /// emitted as `region`/`country`.
    fn country_code_corroboration(
        &self,
        result: &mut ColumnResult,
        _header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("country_code_corroboration") {
            return;
        }
        const CONFUSABLE: [&str; 3] = [
            "geography.location.region",
            "geography.location.city",
            "geography.location.country",
        ];
        if !CONFUSABLE.contains(&result.label.as_str()) {
            return;
        }
        let Some(taxonomy) = self.taxonomy.as_ref() else {
            return;
        };
        let Some(validator) = taxonomy.get_validator("geography.location.country_code") else {
            return;
        };
        let non_empty = non_empty_trimmed(sample);
        if non_empty.len() < 3 {
            return;
        }
        let pass = non_empty.iter().filter(|v| validator.is_valid(v)).count();
        // Promote only on a clear majority of exact ISO codes.
        if pass * 2 <= non_empty.len() {
            return;
        }
        result.label = "geography.location.country_code".to_string();
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some("country_code_corroboration".to_string());
        result.detected_locale = None;
    }

    /// `geo_code_membership_vote` (default ON). Dominant-member vote for the
    /// `state_code` <-> `country_code` confusion, which `country_code_corroboration`
    /// deliberately does NOT touch: 31 of the 56 US-state codes (AL, CA, DE, IN,
    /// …) are ALSO ISO-3166-1 country codes, so a genuine US-states column already
    /// scores ~0.51 country coverage — a simple >50% majority would wrongly promote
    /// it. This is the set-vs-set case membership sets exist for (certainties, not
    /// simulated semantics): count each value's membership in the country_code enum
    /// vs the union of state_code's subdivision-locale enums (US/CA/AU) and assign
    /// the winner ONLY when it clears a high bar (>=0.70) AND beats the loser by a
    /// margin (>=0.20). A genuinely ambiguous column (all values valid as both) fails
    /// the margin and keeps the model's label.
    ///
    /// Motivating case (dataset-descriptor audit): GLEIF `jurisdiction` is 89% ISO
    /// country codes (IN, IT, DE, GB) + 11% ISO-3166-2 subdivisions (US-DE) — the
    /// 11% tail drags the model to `state_code`; measured coverage country 0.89 vs
    /// us_state 0.21 votes it back to `country_code`. A genuine US-states column
    /// (state 1.0, country 0.51) votes `state_code`. Value-based (0048), RHH-disableable.
    /// Reads existing taxonomy enums — no new set files.
    fn geo_code_membership_vote(
        &self,
        result: &mut ColumnResult,
        _header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("geo_code_membership_vote") {
            return;
        }
        const COUNTRY: &str = "geography.location.country_code";
        const STATE: &str = "geography.location.state_code";
        if result.label != COUNTRY && result.label != STATE {
            return;
        }
        let Some(taxonomy) = self.taxonomy.as_ref() else {
            return;
        };
        let Some(country_v) = taxonomy.get_validator(COUNTRY) else {
            return;
        };
        // Subdivision substance is the UNION of every state_code locale enum
        // (EN_US 56 + EN_CA 13 + EN_AU 8), not the top-level `^[A-Z]{2}$` shape and
        // not US alone: Canadian province codes (NL, SK, YT, QC…) collide with
        // country codes (Netherlands, Slovakia, Mayotte), so a US-only set would
        // score a Canadian-province column ~0.5 country / low state and wrongly
        // vote it country_code. Unioning all supported locales counts it as a
        // subdivision column (measured: CA provinces score subdiv 1.0 / country 0.5).
        let subdivisions: std::collections::HashSet<String> = taxonomy
            .get(STATE)
            .and_then(|d| d.validation_by_locale.as_ref())
            .map(|by_locale| {
                by_locale
                    .values()
                    .filter_map(|v| v.enum_values.as_ref())
                    .flatten()
                    .map(|s| s.to_uppercase())
                    .collect()
            })
            .unwrap_or_default();
        if subdivisions.is_empty() {
            return;
        }
        let non_empty: Vec<String> = sample
            .iter()
            .map(|v| v.trim().to_uppercase())
            .filter(|v| !v.is_empty())
            .collect();
        if non_empty.len() < 3 {
            return;
        }
        let n = non_empty.len() as f64;
        let country_cov = non_empty.iter().filter(|v| country_v.is_valid(v)).count() as f64 / n;
        let state_cov = non_empty
            .iter()
            .filter(|v| subdivisions.contains(*v))
            .count() as f64
            / n;
        const WIN: f64 = 0.70;
        const MARGIN: f64 = 0.20;
        let winner = if country_cov >= WIN && country_cov >= state_cov + MARGIN {
            COUNTRY
        } else if state_cov >= WIN && state_cov >= country_cov + MARGIN {
            STATE
        } else {
            // No clear dominant member (e.g. a column of codes valid as both) —
            // leave the model's call rather than guess.
            return;
        };
        if winner == result.label {
            return;
        }
        let from = result.label.clone();
        result.label = winner.to_string();
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("geo_code_membership_vote:{from}"));
        result.detected_locale = detect_locale_from_validation(sample, &result.label, taxonomy);
    }

    /// `geo_code_nonmembership_demotion` (default ON). The set-vs-set companion to
    /// `geo_code_membership_vote`: the vote ARBITRATES a `state_code` <-> `country_code`
    /// column but ABSTAINS — keeps the model's label — when NEITHER set covers the
    /// column. That abstention leaves the wrong label on a NON-geographic short-code
    /// column the flat softmax pulled onto a 2-letter geo type: `work_type` (OT/EQ/MH),
    /// `permit_type` (EW/FO/DM), `ticker` (NVDA). These PASS `state_code`/`country_code`'s
    /// shape-only `^[A-Z]{2}$` validator (so `schema_fail_demotion`, which keys on
    /// validator failure, cannot touch them) yet are not location codes. Membership is
    /// the discriminator: measure coverage against the country enum UNION the state
    /// subdivision enums — counting bare codes AND the ISO-3166-2 hyphenated form
    /// (`US-MA`), because a genuine subdivision column in hyphenated form scores 0% on
    /// the bare set and would otherwise be FALSE-demoted (the gleif `region` control).
    /// When < 50% of values are ANY location code, demote to the cardinality residual
    /// (`word` for a small vocabulary, `alphanumeric_id` for high-card) — matching
    /// `schema_fail_demotion`. Measured (external band): work_type 39% / permit_type 43%
    /// / permit_subtype 26% / ticker 9% coverage vs genuine country 100% / jurisdiction
    /// 100% / region 100% — a 0.50 floor separates them cleanly. A SHAPE GATE fires the
    /// guard ONLY on the 2-letter `^[A-Z]{2}$` attractor form: a genuine country column
    /// in ALPHA-3 (`FRA`) or numeric form is a real geography type the enum doesn't list,
    /// and demoting it is a false-positive (the gold `primaryCountry.alpha3Code` /
    /// `Country Code` regression, caught by a gold-flat re-check). Value-based (0048),
    /// demote-only, RHH-disableable. KNOWN LIMIT: a genuine state column from a locale
    /// outside the US/CA/AU subdivision enums (e.g. German Bundesland codes) scores low
    /// and would demote — expanding the subdivision roster is the fix, not relaxing this.
    fn geo_code_nonmembership_demotion(
        &self,
        result: &mut ColumnResult,
        _header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("geo_code_nonmembership_demotion") {
            return;
        }
        const COUNTRY: &str = "geography.location.country_code";
        const STATE: &str = "geography.location.state_code";
        if result.label != COUNTRY && result.label != STATE {
            return;
        }
        let Some(taxonomy) = self.taxonomy.as_ref() else {
            return;
        };
        // Build the country + subdivision sets directly from the taxonomy enums —
        // the country enum from `validation.enum_values`, the subdivision union the
        // same way `geo_code_membership_vote` builds it. (The full taxonomy carries
        // the complete ISO-3166-1 list; the inline test fixture a curated subset.)
        let country_codes: std::collections::HashSet<String> = taxonomy
            .get(COUNTRY)
            .and_then(|d| d.validation.as_ref())
            .and_then(|val| val.enum_values.as_ref())
            .map(|e| e.iter().map(|s| s.to_uppercase()).collect())
            .unwrap_or_default();
        let subdivisions: std::collections::HashSet<String> = taxonomy
            .get(STATE)
            .and_then(|d| d.validation_by_locale.as_ref())
            .map(|by_locale| {
                by_locale
                    .values()
                    .filter_map(|v| v.enum_values.as_ref())
                    .flatten()
                    .map(|s| s.to_uppercase())
                    .collect()
            })
            .unwrap_or_default();
        if country_codes.is_empty() || subdivisions.is_empty() {
            return;
        }
        let non_empty: Vec<String> = sample
            .iter()
            .map(|v| v.trim().to_uppercase())
            .filter(|v| !v.is_empty())
            .collect();
        if non_empty.len() < 3 {
            return;
        }
        // Shape gate: only fire on the 2-letter attractor shape `^[A-Z]{2}$` — the
        // exact form that pulls a NON-geo short code onto `state_code`/`country_code`
        // (work_type OT/EQ, permit_type EW/FO). A genuine country column in ALPHA-3
        // (`FRA`, `USA`) or numeric form is NOT this shape and is a real geography
        // type the enum simply doesn't list — demoting it is a false-positive (the
        // gold `primaryCountry.alpha3Code` / `Country Code` control). Requiring the
        // 2-letter majority leaves those columns untouched.
        let two_letter = non_empty
            .iter()
            .filter(|v| v.len() == 2 && v.chars().all(|c| c.is_ascii_uppercase()))
            .count();
        if two_letter * 2 < non_empty.len() {
            return;
        }
        // "Any location code" = a country-enum member, a bare subdivision code, or an
        // ISO-3166-2 hyphenated subdivision (`US-MA`) whose prefix is a real country.
        // The hyphenated arm is load-bearing: without it a genuine `US-MA` region
        // column scores 0% and is wrongly demoted.
        let is_geo_code = |v: &str| -> bool {
            if country_codes.contains(v) || subdivisions.contains(v) {
                return true;
            }
            if let Some((cc, sub)) = v.split_once('-') {
                if cc.len() == 2
                    && !sub.is_empty()
                    && sub.len() <= 3
                    && sub.chars().all(|c| c.is_ascii_alphanumeric())
                    && country_codes.contains(cc)
                {
                    return true;
                }
            }
            false
        };
        let geo_cov =
            non_empty.iter().filter(|v| is_geo_code(v)).count() as f64 / non_empty.len() as f64;
        const FLOOR: f64 = 0.50;
        if geo_cov >= FLOOR {
            // Enough real location codes -> genuine geography column, leave it.
            return;
        }
        // Not a location-code column. Demote to the cardinality residual, matching
        // `schema_fail_demotion` (small vocabulary -> word; high-card -> alphanumeric_id).
        let mut distinct: Vec<&str> = non_empty.iter().map(String::as_str).collect();
        distinct.sort_unstable();
        distinct.dedup();
        let residual = if (1..=20).contains(&distinct.len()) {
            "representation.text.word"
        } else {
            "representation.identifier.alphanumeric_id"
        };
        let from = result.label.clone();
        result.label = residual.to_string();
        result.confidence = result.confidence.min(0.6);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("geo_code_nonmembership_demotion:{from}"));
        result.detected_locale = None;
    }

    /// `geo_subdivision_membership_promote` (default ON). The promote companion to
    /// the geo membership guards: a column of ISO-3166-2 subdivision codes
    /// (`US-PA`, `GB-ENG`) is a `geography.location.region`, but the flat softmax —
    /// which never learned the hyphenated form — files it under a residual
    /// (`alphanumeric_id`/`unknown`) or a lookalike (`last_name`, `locale_code`),
    /// and the unlocode `membership_substance_guard` then demotes the model's
    /// `unlocode` guess to `unknown` (external band: ourairports `iso_region`
    /// US-PA → unknown). The `CC-SSS` shape is NOT precise — product/OS/locale
    /// hyphen-codes share it (100 alphanumeric_id + 16 os + 13 entity columns at
    /// corpus scale) — so this keys on published ISO-3166-2 MEMBERSHIP
    /// (`finetype_core::membership::iso_3166_2`, labels/sets/iso_3166_2_codes.txt,
    /// 5046 codes across 200 countries). When ≥90% of values are real subdivision
    /// codes, promote to region. Membership at that density is self-precise (a
    /// non-geo column is ~never 90% exact subdivision codes), so it fires on ANY
    /// source label except an already-correct region — mirroring the naics /
    /// s_expression recoveries. Value-based (0048), promote-only, RHH-disableable.
    fn geo_subdivision_membership_promote(
        &self,
        result: &mut ColumnResult,
        _header: &str,
        sample: &[String],
    ) {
        if rhh::is_disabled("geo_subdivision_membership_promote") {
            return;
        }
        const REGION: &str = "geography.location.region";
        // Already the target — nothing to promote.
        if result.label == REGION {
            return;
        }
        let non_empty: Vec<&str> = sample
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .collect();
        if non_empty.len() < 3 {
            return;
        }
        let members = non_empty
            .iter()
            .filter(|v| finetype_core::membership::iso_3166_2(v))
            .count();
        // ≥90% published ISO-3166-2 membership — self-precise at this density, so
        // the source label is irrelevant (a non-geo column is ~never 90% exact
        // subdivision codes). Shape alone would false-promote hyphenated
        // product/OS/locale codes; membership is the discriminator.
        if members * 10 < non_empty.len() * 9 {
            return;
        }
        let from = result.label.clone();
        result.label = REGION.to_string();
        result.confidence = result.confidence.max(0.85);
        result.disambiguation_applied = true;
        result.disambiguation_rule = Some(format!("geo_subdivision_membership_promote:{from}"));
        result.detected_locale = None;
    }

    /// `amount_bare_number_veto` (default ON). Runs AFTER `apply_header_sharpen`,
    /// from the caller, because the currency.amount over-emission is CREATED inside
    /// the header-hint machinery via an early `return` (a money-ish header promotes
    /// the model's correct integer/decimal to currency.amount — P=0.105 on gold),
    /// which a guard *within* apply_header_sharpen cannot reach. The header cannot
    /// disambiguate (genuine amounts — base_salary, price — carry money headers
    /// too), but a genuine currency.amount carries a currency signal (£45.17,
    /// EUR 4 459 807) while the false positives are bare numbers (netIncome
    /// 795000000). So undo the hint's promotion by value shape: a bare-number
    /// currency.amount is demoted back to decimal/integer. Demotion only.
    fn amount_bare_number_veto(&self, result: &mut ColumnResult, header: &str, sample: &[String]) {
        if rhh::is_disabled("amount_bare_number_veto") || result.label != "finance.currency.amount"
        {
            return;
        }
        let (is_bare, any_decimal) = values_look_like_bare_numbers(sample);
        if is_bare {
            demote_bare_to_numeric(
                result,
                sample,
                self.taxonomy.as_ref(),
                any_decimal,
                "amount_bare_number_veto",
                header,
            );
        }
    }
}

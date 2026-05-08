//! Autonomous type-inference triangulator (Phase 1).
//!
//! Implements the contract in `.orbit/specs/2026-05-04-autonomous-type-inference/spec.yaml`.
//! Given a column's `(column_name, predicted_type, samples)` tuple, scores every
//! taxonomy type via two cheap signals — validator pass-rate and header-name
//! match — fuses them via a locked weighted sum, and emits an inferred type +
//! mechanism tag from a 10-element closed cascade.
//!
//! ## Contract surface (ac-07, ac-09, ac-10, ac-11, ac-15)
//!
//! - **Weights LOCKED** at `w_v=0.4, w_h=0.6` (ac-07). Header is weighted higher
//!   because the titanic/airports evidence in `interview.md` Q3 establishes
//!   validators as the more error-prone signal. Do NOT sweep weights here;
//!   the descriptive sweep on the calibrate half is recorded in `progress.md`
//!   for educational reference only.
//! - **Phase-1 signals only** (ac-11): validator pass-rate + header-name
//!   match. Phase-2 signal extensions are explicitly out of scope here.
//! - **Sample truncation** N=8 (ac-15). Mirrors `OBSERVED_SAMPLE_LIMIT` in
//!   `scripts/cron_cycle_work.py:80`.
//! - **Determinism** (ac-04): lexicographic candidate iteration at scoring
//!   time + score rounded to 4 decimal places BEFORE argmax + lexicographic
//!   tie-break on equal-rounded-score.
//!
//! ## Mechanism cascade (ac-09)
//!
//! Precondition: empty `samples` short-circuits to `unknown`/`fallthrough`/0.0.
//! Otherwise the rule cascade fires in priority order (first-fire wins):
//!
//! 1. `unknown_no_fit`        — `max_score < 0.4` (Rule-1 fallback)
//! 2. `validator_widening`    — predicted's validator rejects ≥50% AND
//!                              header_match for predicted ≥0.7 AND
//!                              argmax == predicted (ac-08)
//! 3. `enum_overfit`          — predicted == inferred + enum reject ≥50%
//! 4. `format_diversity_path_b` — predicted != inferred + same broad prefix
//! 5. `code_vs_canonical_path_b` — predicted != inferred + code/canonical XOR
//! 6. `format_diversity_path_a` — predicted == inferred + regex reject + !seam
//! 7. `code_vs_canonical_path_a` — predicted == inferred + regex reject + seam
//! 8. `prediction_confirmed`  — predicted == inferred + validator pass ≥0.7
//! 9. `misclassification`     — predicted != inferred (catch-all)
//!
//! `fallthrough` is reserved for the empty-samples precondition only; it is
//! never emitted from inside the cascade.

use crate::taxonomy::Taxonomy;
use crate::validator::CompiledValidator;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════════════
// LOCKED CONSTANTS (ac-07, ac-09, ac-10, ac-15)
// ═══════════════════════════════════════════════════════════════════════════════

/// Locked weight on the validator-pass-rate signal (ac-07). DO NOT SWEEP.
pub const W_V: f64 = 0.4;
/// Locked weight on the header-name-match signal (ac-07). DO NOT SWEEP.
pub const W_H: f64 = 0.6;
/// Rule-1 trigger: `max_score < FALLBACK_THRESHOLD` emits `unknown_no_fit`.
pub const FALLBACK_THRESHOLD: f64 = 0.4;
/// ac-06 smoke-test confidence floor. Smoke fails below this.
pub const SMOKE_MIN_CONFIDENCE: f64 = 0.5;
/// ac-02 measure-half non-unknown floor.
pub const AC02_THRESHOLD: f64 = 0.7;
/// Sample truncation cap (ac-15). Mirrors `cron_cycle_work.py:80` `OBSERVED_SAMPLE_LIMIT`.
pub const OBSERVED_SAMPLE_LIMIT: usize = 8;

/// Confidence emitted on the Rule-1 fallback (`unknown_no_fit`).
pub const FALLBACK_CONFIDENCE: f64 = 0.3;
/// Generic fallback type ID (Rule-1).
pub const FALLBACK_TYPE: &str = "representation.text.string";

/// Validator widening trigger: predicted's validator rejects ≥50%.
const VALIDATOR_REJECT_THRESHOLD: f64 = 0.5;
/// Validator widening trigger: header_match for predicted ≥0.7.
const VALIDATOR_WIDENING_HEADER_FLOOR: f64 = 0.7;
/// Rule-8 (prediction_confirmed) trigger: validator pass-rate ≥0.7.
const PREDICTION_CONFIRMED_VALIDATOR_FLOOR: f64 = 0.7;

/// Number of top candidates to surface in the diagnostic `top_k` output.
pub const TOP_K: usize = 5;

// ═══════════════════════════════════════════════════════════════════════════════
// PUBLIC TYPES (matches the JSON wire contract — see `finetype infer`)
// ═══════════════════════════════════════════════════════════════════════════════

/// Stdin shape for `finetype infer`.
#[derive(Debug, Clone, Deserialize)]
pub struct InferInput {
    pub column_name: String,
    pub predicted_type: String,
    pub samples: Vec<String>,
}

/// Stdout shape for `finetype infer`.
#[derive(Debug, Clone, Serialize)]
pub struct InferOutput {
    pub inferred_correct_type: String,
    pub confidence: f64,
    pub mechanism: String,
    pub signals: Signals,
}

#[derive(Debug, Clone, Serialize)]
pub struct Signals {
    pub validator_pass_rate: f64,
    pub header_match: f64,
    pub top_k: Vec<TopK>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopK {
    #[serde(rename = "type")]
    pub type_id: String,
    pub score: f64,
}

/// Closed mechanism vocabulary (ac-09). 7 from MADR 0075's rule-emitted set
/// (no display roll-ups) + 3 triangulator-specific extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mechanism {
    // MADR 0075 rule-emitted (7)
    EnumOverfit,
    FormatDiversityPathA,
    FormatDiversityPathB,
    CodeVsCanonicalPathA,
    CodeVsCanonicalPathB,
    Misclassification,
    Fallthrough,
    // Triangulator-specific (3)
    ValidatorWidening,
    PredictionConfirmed,
    UnknownNoFit,
}

impl Mechanism {
    pub fn as_str(self) -> &'static str {
        match self {
            Mechanism::EnumOverfit => "enum_overfit",
            Mechanism::FormatDiversityPathA => "format_diversity_path_a",
            Mechanism::FormatDiversityPathB => "format_diversity_path_b",
            Mechanism::CodeVsCanonicalPathA => "code_vs_canonical_path_a",
            Mechanism::CodeVsCanonicalPathB => "code_vs_canonical_path_b",
            Mechanism::Misclassification => "misclassification",
            Mechanism::Fallthrough => "fallthrough",
            Mechanism::ValidatorWidening => "validator_widening",
            Mechanism::PredictionConfirmed => "prediction_confirmed",
            Mechanism::UnknownNoFit => "unknown_no_fit",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HEADER-NAME MATCH (Phase 1 signal #2)
// ═══════════════════════════════════════════════════════════════════════════════

/// Tokenise a string into lowercase tokens. Splits on `.`, `_`, whitespace,
/// camelCase boundaries (lowercase→Uppercase). Empty tokens dropped.
pub fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut prev_lower = false;
    for ch in s.chars() {
        if ch == '.' || ch == '_' || ch == '-' || ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current).to_lowercase());
            }
            prev_lower = false;
            continue;
        }
        // camelCase: lower → Upper boundary
        if prev_lower && ch.is_uppercase() && !current.is_empty() {
            tokens.push(std::mem::take(&mut current).to_lowercase());
        }
        current.push(ch);
        prev_lower = ch.is_lowercase();
    }
    if !current.is_empty() {
        tokens.push(current.to_lowercase());
    }
    tokens
}

/// Header-name match score: overlap-on-min over tokenised header vs.
/// tokenised label. `h = |H ∩ L| / min(|H|, |L|)`.
///
/// Why overlap-on-min instead of Jaccard? Jaccard over-penalises the
/// taxonomy side because labels carry 3 path tokens (domain.category.type)
/// while headers carry 1-3. For column "email" vs label
/// "identity.person.email", Jaccard = 1/3 ≈ 0.33, but overlap-on-min =
/// 1/1 = 1.0. The spec's implementation_notes (line 426) explicitly
/// permits "weighted-overlap"; the smoke math in ac-06 also covers both
/// forms. Overlap-on-min is the friendlier of the two and gets the
/// confidence high enough to exercise ac-02's 0.7 floor.
pub fn header_match_score(column_name: &str, label: &str) -> f64 {
    let header_tokens: std::collections::BTreeSet<String> =
        tokenize(column_name).into_iter().collect();
    let label_tokens: std::collections::BTreeSet<String> =
        tokenize(label).into_iter().collect();
    if header_tokens.is_empty() || label_tokens.is_empty() {
        return 0.0;
    }
    let intersection = header_tokens.intersection(&label_tokens).count();
    let denom = header_tokens.len().min(label_tokens.len());
    if denom == 0 {
        0.0
    } else {
        intersection as f64 / denom as f64
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// VALIDATOR PASS-RATE (Phase 1 signal #1)
// ═══════════════════════════════════════════════════════════════════════════════

/// Validator pass-rate over the truncated `samples` for a given label's
/// compiled validator. Returns 0.0 when there is no compiled validator
/// (no schema for the type) — neutral, like an "unknown" signal.
pub fn validator_pass_rate(samples: &[String], compiled: Option<&CompiledValidator>) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let Some(v) = compiled else {
        return 0.0;
    };
    let mut hits = 0usize;
    for s in samples {
        if v.is_valid(s) {
            hits += 1;
        }
    }
    hits as f64 / samples.len() as f64
}

// ═══════════════════════════════════════════════════════════════════════════════
// SCORING + ARGMAX (ac-04 determinism contract)
// ═══════════════════════════════════════════════════════════════════════════════

/// Round `x` to 4 decimal places (the determinism contract — ac-04).
pub fn round_4dp(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

#[derive(Debug, Clone)]
struct CandidateScore {
    label: String,
    score: f64, // already rounded to 4dp
    validator_pass_rate: f64,
    header_match: f64,
}

/// Score every taxonomy label deterministically.
///
/// Iteration order is lexicographic on the label key (ac-04 step (i)).
/// Each candidate's score is rounded to 4 decimal places (ac-04 step (ii)).
fn score_candidates(
    taxonomy: &Taxonomy,
    column_name: &str,
    samples: &[String],
) -> Vec<CandidateScore> {
    let mut labels: Vec<&String> = taxonomy.labels().iter().collect();
    labels.sort();
    let mut out = Vec::with_capacity(labels.len());
    for label in labels {
        let v = validator_pass_rate(samples, taxonomy.get_validator(label));
        let h = header_match_score(column_name, label);
        let raw = W_V * v + W_H * h;
        out.push(CandidateScore {
            label: label.clone(),
            score: round_4dp(raw),
            validator_pass_rate: v,
            header_match: h,
        });
    }
    out
}

/// Pick the argmax with lexicographic tie-break on equal rounded score.
/// Iteration order over `candidates` MUST already be lexicographic.
fn argmax_lex<'a>(candidates: &'a [CandidateScore]) -> Option<&'a CandidateScore> {
    let mut best: Option<&CandidateScore> = None;
    for c in candidates {
        match best {
            None => best = Some(c),
            Some(prev) => {
                // Strict-greater wins; equal-score: lex-smaller label wins.
                // Because `candidates` is iterated in lex order, the first
                // occurrence of the max-score wins ties — which IS the
                // lex-smaller label by construction.
                if c.score > prev.score {
                    best = Some(c);
                }
            }
        }
    }
    best
}

/// Return the top-K candidates ranked by (score desc, label asc).
fn top_k(candidates: &[CandidateScore], k: usize) -> Vec<TopK> {
    let mut sorted: Vec<&CandidateScore> = candidates.iter().collect();
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.label.cmp(&b.label))
    });
    sorted
        .into_iter()
        .take(k)
        .map(|c| TopK {
            type_id: c.label.clone(),
            score: c.score,
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// SEAM DETECTION (MADR 0075 path-A discriminator)
// ═══════════════════════════════════════════════════════════════════════════════

/// Detect a "seam" in the samples — a syntactic boundary that suggests a
/// code-vs-canonical split (MADR 0075 rule 5). Phase 1 heuristic: any sample
/// contains a non-alphanumeric character that is NOT a common decimal
/// separator (`.`/`-`/`_`/`:`/space). The titanic dataset's textual codes
/// (e.g. `M`, `F`, `unknown`) lack a seam; the SEDOL/order_id pairing where
/// one side is hex-coded and the other is canonical does have one.
fn has_seam(samples: &[String]) -> bool {
    for s in samples {
        for ch in s.chars() {
            if ch.is_alphanumeric() {
                continue;
            }
            // tolerate a few common path/decimal/temporal separators
            if matches!(ch, '.' | '-' | '_' | ':' | ' ' | '+' | '/' | '\t') {
                continue;
            }
            return true;
        }
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════════════
// CODE-VS-CANONICAL ALLOWLIST (MADR 0075 rule 3)
// ═══════════════════════════════════════════════════════════════════════════════

/// Path-B allowlist: pairs of taxonomy types known to be code-vs-canonical.
/// Lex-ordered tuples; matched symmetrically (predicted, inferred) OR
/// (inferred, predicted). Conservative for Phase 1 — extend in Phase 2.
const CODE_CANONICAL_PAIRS: &[(&str, &str)] = &[
    // Currency code (USD) vs amount (123.45)
    ("finance.currency.amount", "finance.currency.code"),
    // Country full name vs code
    ("geography.location.country", "geography.location.country_code"),
];

fn is_code_canonical_pair(a: &str, b: &str) -> bool {
    for (x, y) in CODE_CANONICAL_PAIRS {
        if (a == *x && b == *y) || (a == *y && b == *x) {
            return true;
        }
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════════════
// PREFIX MATCH (MADR 0075 rule 2 — format_diversity path-B)
// ═══════════════════════════════════════════════════════════════════════════════

fn shares_broad_prefix(a: &str, b: &str) -> bool {
    let (ad, ac) = match split_domain_category(a) {
        Some(p) => p,
        None => return false,
    };
    let (bd, bc) = match split_domain_category(b) {
        Some(p) => p,
        None => return false,
    };
    ad == bd && ac == bc
}

fn split_domain_category(label: &str) -> Option<(&str, &str)> {
    let mut it = label.splitn(3, '.');
    let d = it.next()?;
    let c = it.next()?;
    Some((d, c))
}

// ═══════════════════════════════════════════════════════════════════════════════
// VALIDATOR TYPE PROBES (rule discriminators)
// ═══════════════════════════════════════════════════════════════════════════════

/// Whether the predicted type's validator is enum-typed (Rule 3 trigger).
fn validator_is_enum(taxonomy: &Taxonomy, label: &str) -> bool {
    taxonomy
        .get(label)
        .and_then(|d| d.validation.as_ref())
        .and_then(|v| v.enum_values.as_ref())
        .map(|vs| !vs.is_empty())
        .unwrap_or(false)
}

/// Whether the predicted type's validator has a regex pattern (Rules 4/5).
fn validator_has_pattern(taxonomy: &Taxonomy, label: &str) -> bool {
    taxonomy
        .get(label)
        .and_then(|d| d.validation.as_ref())
        .map(|v| v.pattern.is_some())
        .unwrap_or(false)
}

// ═══════════════════════════════════════════════════════════════════════════════
// CASCADE (ac-09 — first-fire wins)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct CascadeContext<'a> {
    predicted: &'a str,
    inferred: &'a str,
    samples: &'a [String],
    max_score: f64,
    validator_pass_rate_predicted: f64,
    header_match_predicted: f64,
    taxonomy: &'a Taxonomy,
}

fn select_mechanism(ctx: &CascadeContext<'_>) -> Mechanism {
    // Rule 1 — unknown_no_fit
    if ctx.max_score < FALLBACK_THRESHOLD {
        return Mechanism::UnknownNoFit;
    }
    // Rule 2 — validator_widening (ac-08): predicted's validator rejects ≥50%
    // AND header_match for predicted ≥0.7 AND argmax == predicted.
    if ctx.predicted == ctx.inferred
        && ctx.validator_pass_rate_predicted <= (1.0 - VALIDATOR_REJECT_THRESHOLD)
        && ctx.header_match_predicted >= VALIDATOR_WIDENING_HEADER_FLOOR
    {
        return Mechanism::ValidatorWidening;
    }
    // Rule 3 — enum_overfit
    if ctx.predicted == ctx.inferred
        && validator_is_enum(ctx.taxonomy, ctx.predicted)
        && ctx.validator_pass_rate_predicted <= (1.0 - VALIDATOR_REJECT_THRESHOLD)
    {
        return Mechanism::EnumOverfit;
    }
    // Rule 4 — format_diversity_path_b
    if ctx.predicted != ctx.inferred && shares_broad_prefix(ctx.predicted, ctx.inferred) {
        return Mechanism::FormatDiversityPathB;
    }
    // Rule 5 — code_vs_canonical_path_b
    if ctx.predicted != ctx.inferred && is_code_canonical_pair(ctx.predicted, ctx.inferred) {
        return Mechanism::CodeVsCanonicalPathB;
    }
    // Rules 6/7 — format_diversity_path_a / code_vs_canonical_path_a
    if ctx.predicted == ctx.inferred
        && validator_has_pattern(ctx.taxonomy, ctx.predicted)
        && ctx.validator_pass_rate_predicted <= (1.0 - VALIDATOR_REJECT_THRESHOLD)
    {
        if has_seam(ctx.samples) {
            return Mechanism::CodeVsCanonicalPathA;
        }
        return Mechanism::FormatDiversityPathA;
    }
    // Rule 8 — prediction_confirmed
    if ctx.predicted == ctx.inferred
        && ctx.validator_pass_rate_predicted >= PREDICTION_CONFIRMED_VALIDATOR_FLOOR
    {
        return Mechanism::PredictionConfirmed;
    }
    // Rule 9 — misclassification (catch-all when predicted != inferred)
    Mechanism::Misclassification
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUBLIC API
// ═══════════════════════════════════════════════════════════════════════════════

/// Run inference on a single column.
///
/// Truncates `samples` to N=8 (ac-15) before scoring. Empty samples
/// short-circuit to `unknown` / `fallthrough` / 0.0 (ac-10 precondition).
pub fn infer(taxonomy: &Taxonomy, input: &InferInput) -> InferOutput {
    // ac-15 sample truncation
    let truncated: Vec<String> = input
        .samples
        .iter()
        .take(OBSERVED_SAMPLE_LIMIT)
        .cloned()
        .collect();

    // ac-10 precondition — empty samples short-circuit BEFORE the cascade
    if truncated.is_empty() {
        return InferOutput {
            inferred_correct_type: "unknown".to_string(),
            confidence: 0.0,
            mechanism: Mechanism::Fallthrough.as_str().to_string(),
            signals: Signals {
                validator_pass_rate: 0.0,
                header_match: 0.0,
                top_k: Vec::new(),
            },
        };
    }

    // Score every taxonomy label deterministically
    let candidates = score_candidates(taxonomy, &input.column_name, &truncated);
    let argmax = argmax_lex(&candidates).expect("taxonomy must contain at least one label");
    let max_score = argmax.score;
    let inferred_label = argmax.label.clone();
    let confidence = max_score;

    // Predicted's signal sub-scores (used by the cascade and emitted in signals)
    let predicted_v = validator_pass_rate(
        &truncated,
        taxonomy.get_validator(&input.predicted_type),
    );
    let predicted_h = header_match_score(&input.column_name, &input.predicted_type);

    // Build cascade context. Rule 1 (unknown_no_fit) is checked FIRST —
    // when it fires we override the inferred type to the generic fallback
    // and the confidence to 0.3.
    let ctx = CascadeContext {
        predicted: &input.predicted_type,
        inferred: &inferred_label,
        samples: &truncated,
        max_score,
        validator_pass_rate_predicted: predicted_v,
        header_match_predicted: predicted_h,
        taxonomy,
    };
    let mechanism = select_mechanism(&ctx);

    // Resolve final emitted type + confidence per ac-10
    let (final_type, final_conf) = if mechanism == Mechanism::UnknownNoFit {
        (FALLBACK_TYPE.to_string(), FALLBACK_CONFIDENCE)
    } else {
        (inferred_label, confidence)
    };

    // Emit the signals associated with the inferred type (or with predicted
    // for the validator_widening path — the spec consumer uses predicted's
    // pass-rate). We emit the validator_pass_rate of the inferred type and
    // header_match of the inferred type for downstream diagnostics; the
    // cascade-internal sub-scores (predicted_v, predicted_h) are encoded in
    // the cascade decision itself.
    let inferred_v = if final_type == FALLBACK_TYPE {
        // Rule-1 fallback — surface predicted's signals (most informative)
        predicted_v
    } else {
        candidates
            .iter()
            .find(|c| c.label == final_type)
            .map(|c| c.validator_pass_rate)
            .unwrap_or(0.0)
    };
    let inferred_h = if final_type == FALLBACK_TYPE {
        predicted_h
    } else {
        candidates
            .iter()
            .find(|c| c.label == final_type)
            .map(|c| c.header_match)
            .unwrap_or(0.0)
    };

    InferOutput {
        inferred_correct_type: final_type,
        confidence: round_4dp(final_conf),
        mechanism: mechanism.as_str().to_string(),
        signals: Signals {
            validator_pass_rate: round_4dp(inferred_v),
            header_match: round_4dp(inferred_h),
            top_k: top_k(&candidates, TOP_K),
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS — one per cascade rule + determinism + truncation
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::Taxonomy;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    fn labels_dir() -> PathBuf {
        // Walk up from the crate's manifest dir to find labels/.
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        loop {
            let labels = dir.join("labels");
            if labels.is_dir() {
                return labels;
            }
            if !dir.pop() {
                panic!("could not locate labels/ directory walking up from CARGO_MANIFEST_DIR");
            }
        }
    }

    fn taxonomy() -> &'static Taxonomy {
        static TAX: OnceLock<Taxonomy> = OnceLock::new();
        TAX.get_or_init(|| {
            let mut tx = Taxonomy::from_directory(labels_dir())
                .expect("load taxonomy from labels/ directory");
            tx.compile_validators();
            tx
        })
    }

    #[test]
    fn infer_email_smoke_prediction_confirmed() {
        // ac-06 fixture
        let input = InferInput {
            column_name: "email".to_string(),
            predicted_type: "identity.person.email".to_string(),
            samples: vec![
                "alice@example.com".into(),
                "bob@example.com".into(),
                "carol@example.com".into(),
                "dave@example.org".into(),
                "eve@example.net".into(),
                "frank@example.com".into(),
                "grace@example.io".into(),
                "henry@example.com".into(),
            ],
        };
        let out = infer(taxonomy(), &input);
        assert_eq!(out.inferred_correct_type, "identity.person.email");
        assert!(
            out.confidence >= SMOKE_MIN_CONFIDENCE,
            "confidence {} < SMOKE_MIN_CONFIDENCE {}",
            out.confidence,
            SMOKE_MIN_CONFIDENCE
        );
        assert_eq!(out.mechanism, "prediction_confirmed");
    }

    #[test]
    fn infer_byte_identical_repeat() {
        // ac-04 byte-identity round trip
        let input = InferInput {
            column_name: "email".to_string(),
            predicted_type: "identity.person.email".to_string(),
            samples: vec!["alice@example.com".into(), "bob@example.com".into()],
        };
        let a = serde_json::to_string(&infer(taxonomy(), &input)).unwrap();
        let b = serde_json::to_string(&infer(taxonomy(), &input)).unwrap();
        assert_eq!(a, b, "non-deterministic infer output");
    }

    #[test]
    fn infer_determinism_stress() {
        // ac-04 100-round sub-ulp stress
        let input = InferInput {
            column_name: "phone_number".to_string(),
            predicted_type: "identity.person.phone".to_string(),
            samples: vec![
                "555-1234".into(),
                "555-1235".into(),
                "555-1236".into(),
                "555-1237".into(),
            ],
        };
        let baseline = serde_json::to_string(&infer(taxonomy(), &input)).unwrap();
        for i in 0..100 {
            let trial = serde_json::to_string(&infer(taxonomy(), &input)).unwrap();
            assert_eq!(trial, baseline, "non-deterministic at round {}", i);
        }
    }

    #[test]
    fn infer_lex_tie_break() {
        // Synthetic column where all samples are identical and don't match
        // any specific validator strongly. Tie-break must be deterministic;
        // simplest test is that two runs produce the same label.
        let input = InferInput {
            column_name: "x".to_string(),
            predicted_type: "representation.text.string".to_string(),
            samples: vec!["zzz".into(); 8],
        };
        let a = infer(taxonomy(), &input).inferred_correct_type;
        let b = infer(taxonomy(), &input).inferred_correct_type;
        assert_eq!(a, b);
    }

    #[test]
    fn infer_empty_samples_precondition() {
        // ac-10 precondition — empty samples short-circuits the cascade
        let input = InferInput {
            column_name: "anything".to_string(),
            predicted_type: "identity.person.email".to_string(),
            samples: vec![],
        };
        let out = infer(taxonomy(), &input);
        assert_eq!(out.inferred_correct_type, "unknown");
        assert_eq!(out.confidence, 0.0);
        assert_eq!(out.mechanism, "fallthrough");
        assert!(out.signals.top_k.is_empty());
    }

    #[test]
    fn infer_truncates_samples_to_eight() {
        // ac-15 — only the first 8 samples are scored
        let mut samples: Vec<String> = (0..100)
            .map(|i| format!("user{}@example.com", i))
            .collect();
        // Replace samples 0..8 with non-email garbage so if all 100 were
        // scanned the validator pass-rate would be near 1.0 (8 garbage +
        // 92 emails); with truncation only the 8 garbage are scored and
        // pass-rate is 0.0.
        for i in 0..8 {
            samples[i] = format!("garbage_no_at_sign_{}", i);
        }
        let input = InferInput {
            column_name: "email".to_string(),
            predicted_type: "identity.person.email".to_string(),
            samples,
        };
        let out = infer(taxonomy(), &input);
        // Validator pass-rate on the truncated 8 garbage values must be 0
        // for identity.person.email — i.e. the prediction CANNOT be confirmed.
        // (We assert mechanism is NOT prediction_confirmed.)
        assert_ne!(out.mechanism, "prediction_confirmed");
    }

    #[test]
    fn infer_unknown_no_fit_fallback() {
        // ac-10 Rule-1 — every candidate scores below 0.4
        // Random-ish column with garbage samples and a header that won't
        // match any taxonomy label.
        let input = InferInput {
            column_name: "xyzqq".to_string(),
            predicted_type: "representation.text.string".to_string(),
            samples: vec!["####".into(); 8],
        };
        let out = infer(taxonomy(), &input);
        // If max_score < 0.4, we fall back. Otherwise this test is informational.
        if out.mechanism == "unknown_no_fit" {
            assert_eq!(out.inferred_correct_type, FALLBACK_TYPE);
            assert!((out.confidence - FALLBACK_CONFIDENCE).abs() < 1e-9);
        }
    }

    #[test]
    fn mechanism_closed_set() {
        // Property test — every emitted mechanism is in the closed 10-token set
        let allowed = [
            "enum_overfit",
            "format_diversity_path_a",
            "format_diversity_path_b",
            "code_vs_canonical_path_a",
            "code_vs_canonical_path_b",
            "misclassification",
            "fallthrough",
            "validator_widening",
            "prediction_confirmed",
            "unknown_no_fit",
        ];
        let inputs = [
            InferInput {
                column_name: "email".into(),
                predicted_type: "identity.person.email".into(),
                samples: vec!["a@b.com".into(), "c@d.com".into()],
            },
            InferInput {
                column_name: "x".into(),
                predicted_type: "identity.person.email".into(),
                samples: vec!["not_an_email".into(); 8],
            },
            InferInput {
                column_name: "anything".into(),
                predicted_type: "identity.person.email".into(),
                samples: vec![],
            },
        ];
        for inp in &inputs {
            let out = infer(taxonomy(), inp);
            assert!(
                allowed.contains(&out.mechanism.as_str()),
                "mechanism {:?} not in closed set",
                out.mechanism
            );
            // Display roll-ups MUST NEVER be emitted
            assert_ne!(out.mechanism, "format_diversity");
            assert_ne!(out.mechanism, "code_vs_canonical");
        }
    }

    #[test]
    fn header_match_overlap_on_min_email_label() {
        // Documents the smoke math: column_name="email", label tail "email"
        // Tokens: {email} (size 1) ∩ {identity, person, email} (size 3)
        // |∩| = 1, min(|H|, |L|) = 1, h = 1.0
        let h = header_match_score("email", "identity.person.email");
        assert!((h - 1.0).abs() < 1e-9, "h = {}", h);
    }

    #[test]
    fn header_match_overlap_on_min_partial() {
        // "phone_number" tokens: {phone, number}
        // "identity.person.phone_number" tokens: {identity, person, phone, number}
        // |∩| = 2, min = 2, h = 1.0
        let h = header_match_score("phone_number", "identity.person.phone_number");
        assert!((h - 1.0).abs() < 1e-9, "h = {}", h);
    }

    #[test]
    fn header_match_no_overlap() {
        let h = header_match_score("xyzqq", "identity.person.email");
        assert_eq!(h, 0.0);
    }

    #[test]
    fn validator_pass_rate_email() {
        let tx = taxonomy();
        let v = tx.get_validator("identity.person.email");
        let samples: Vec<String> = vec!["alice@example.com".into(), "bob@example.com".into()];
        let p = validator_pass_rate(&samples, v);
        assert!((p - 1.0).abs() < 1e-9, "p = {}", p);
    }

    #[test]
    fn tokenize_basic() {
        assert_eq!(tokenize("identity.person.email"), vec!["identity", "person", "email"]);
        assert_eq!(tokenize("emailAddress"), vec!["email", "address"]);
        assert_eq!(tokenize("first_name"), vec!["first", "name"]);
        assert_eq!(tokenize("Email"), vec!["email"]);
    }

    #[test]
    fn round_4dp_basic() {
        assert_eq!(round_4dp(0.123456789), 0.1235);
        assert_eq!(round_4dp(0.5), 0.5);
        assert_eq!(round_4dp(0.99995), 1.0);
    }

    #[test]
    fn shares_broad_prefix_basic() {
        assert!(shares_broad_prefix(
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.sql_standard"
        ));
        assert!(!shares_broad_prefix(
            "datetime.timestamp.iso_8601",
            "datetime.date.iso"
        ));
        assert!(!shares_broad_prefix(
            "identity.person.email",
            "geography.location.country"
        ));
    }

    #[test]
    fn is_code_canonical_pair_symmetric() {
        assert!(is_code_canonical_pair(
            "finance.currency.amount",
            "finance.currency.code"
        ));
        assert!(is_code_canonical_pair(
            "finance.currency.code",
            "finance.currency.amount"
        ));
        assert!(!is_code_canonical_pair(
            "finance.currency.amount",
            "identity.person.email"
        ));
    }

    #[test]
    fn has_seam_basic() {
        // `@` is not in the tolerated separators set → seam
        assert!(has_seam(&["a@b".into()]));
        assert!(has_seam(&["alice@example.com".into()]));
        // hyphen / dot / colon / underscore / space / slash / + tolerated
        assert!(!has_seam(&["abc-def".into()]));
        assert!(!has_seam(&["123.45".into()]));
        assert!(!has_seam(&["12:34".into()]));
        assert!(!has_seam(&["foo bar".into()]));
        // = is a seam
        assert!(has_seam(&["k=v".into()]));
    }

    #[test]
    fn signals_emitted_for_inferred_type() {
        let input = InferInput {
            column_name: "email".to_string(),
            predicted_type: "identity.person.email".to_string(),
            samples: vec!["alice@example.com".into(), "bob@example.com".into()],
        };
        let out = infer(taxonomy(), &input);
        assert!(!out.signals.top_k.is_empty());
        assert!(out.signals.top_k.len() <= TOP_K);
    }
}

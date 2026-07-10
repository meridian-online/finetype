use super::*;

/// Disambiguate latitude vs longitude coordinates.
///
/// Rule: If ANY |value| > 90, it must be longitude (latitude max is 90).
///       If ALL |values| ≤ 90, it's likely latitude.
pub(crate) fn disambiguate_coordinates(values: &[String]) -> Option<String> {
    let mut any_over_90 = false;
    let mut all_parseable = true;
    let mut parsed_count = 0;

    for val in values {
        if let Ok(v) = val.trim().parse::<f64>() {
            parsed_count += 1;
            if v.abs() > 90.0 {
                any_over_90 = true;
            }
        } else {
            all_parseable = false;
        }
    }

    // Need at least some parseable values
    if parsed_count < 3 {
        return None;
    }

    if any_over_90 {
        Some("geography.coordinate.longitude".to_string())
    } else if all_parseable {
        // All values within [-90, 90] — likely latitude
        Some("geography.coordinate.latitude".to_string())
    } else {
        None
    }
}
/// Rule: If ≥80% of non-empty values match `\d{1,3}.\d{1,3}.\d{1,3}.\d{1,3}`
/// with each octet in 0..255, classify as ip_v4.
///
/// This prevents the common confusion between IP addresses and version numbers
/// (e.g., "10.0.32.113" looks like a semver to the model).
pub(crate) fn disambiguate_ipv4(values: &[String]) -> Option<String> {
    let non_empty = non_empty_trimmed(values);

    if non_empty.len() < 3 {
        return None;
    }

    let mut ipv4_count = 0;
    for val in &non_empty {
        let parts: Vec<&str> = val.split('.').collect();
        if parts.len() == 4 {
            let all_valid = parts.iter().all(|p| {
                p.parse::<u16>()
                    .map(|n| n <= 255 && !p.is_empty())
                    .unwrap_or(false)
            });
            if all_valid {
                ipv4_count += 1;
            }
        }
    }

    let fraction = ipv4_count as f64 / non_empty.len() as f64;
    if fraction >= 0.8 {
        Some("technology.internet.ip_v4".to_string())
    } else {
        None
    }
}
/// Text length demotion: full_address with long median value length.
///
/// The CharCNN often classifies free-form text (descriptions, recipe steps,
/// paragraphs) as `geography.address.full_address` because addresses and
/// text share features like commas, numbers, and mixed casing. However,
/// real addresses have a median value length around 23 chars while text
/// overcall has a median of 53+ chars.
///
/// Rule: If the top vote is full_address and the median non-empty value
/// length exceeds 100 characters, demote to representation.text.plain_text.
/// Threshold 100 gives 0% false demotion rate on evaluation data.
pub(crate) fn disambiguate_text_length_demotion(
    values: &[String],
    votes: &[(String, usize)],
) -> Option<(String, String)> {
    let top_label = votes.first().map(|(l, _)| l.as_str())?;

    // LENGTH direction ONLY — orthogonal to the litigated short-vocab
    // text_vocab_override NO-GO (R32), which fires on LOW cardinality
    // (distinct 2..=12, distinct/n <= 0.6). Long prose is HIGH cardinality with
    // median length > 100, so the two signals can never co-fire. A real
    // entity_name / word / address is never 100+ chars, so the demotion is
    // safe-by-construction (precedent: 0% false-demotion on the full_address arm).
    const LONG_PROSE_SOURCES: &[&str] = &[
        "geography.address.full_address",
        "representation.text.entity_name",
        "representation.text.word",
    ];
    if !LONG_PROSE_SOURCES.contains(&top_label) {
        return None;
    }

    let mut lengths: Vec<usize> = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(|v| v.len())
        .collect();

    if lengths.len() < 3 {
        return None;
    }

    lengths.sort_unstable();
    let median = lengths[lengths.len() / 2];

    if median > 100 {
        // Keep the existing rule string on the full_address arm (preserves
        // test_text_length_demotion_long_text_as_address); tag the new arms.
        let rule = if top_label == "geography.address.full_address" {
            "text_length_demotion_full_address".to_string()
        } else {
            format!("text_length_demotion_long_prose:{top_label}")
        };
        Some(("representation.text.plain_text".to_string(), rule))
    } else {
        None
    }
}

/// Whitespace guard: full_address demands multi-token values (BACKLOG #8).
///
/// full_address is locale_specific. The locale-robust signal that a value IS an
/// address is internal whitespace — every real address is multi-token (street +
/// locality), regardless of punctuation convention. So this guard is
/// WHITESPACE-ONLY: it deliberately does NOT require commas or street numbers,
/// because that positive-evidence clause false-demotes comma-less foreign
/// addresses (e.g. `Hauptstrasse 12 8001 Zurich`). A correct address always has
/// whitespace, so the guard can never demote a true positive.
pub(crate) fn disambiguate_full_address_whitespace_guard(
    values: &[String],
) -> Option<(String, String)> {
    let non_empty = non_empty_trimmed(values);
    if non_empty.len() < 4 {
        return None;
    }
    let with_space = non_empty
        .iter()
        .filter(|v| v.chars().any(char::is_whitespace))
        .count();
    let space_frac = with_space as f32 / non_empty.len() as f32;
    // Overwhelmingly single-token -> cannot be addresses (0.15 ceiling).
    if space_frac > 0.15 {
        return None;
    }
    // Confirm the alphanumeric_id DESTINATION (letter+digit per value). This only
    // inspects the single-token subset (no real address reaches here) and protects
    // pure-word single tokens from being mis-routed to alphanumeric_id.
    let alnum = non_empty
        .iter()
        .filter(|v| {
            v.chars().any(|c| c.is_ascii_alphabetic()) && v.chars().any(|c| c.is_ascii_digit())
        })
        .count();
    if (alnum as f32 / non_empty.len() as f32) < 0.80 {
        return None;
    }
    Some((
        "representation.identifier.alphanumeric_id".to_string(),
        format!("full_address_whitespace_guard:space_frac={space_frac:.2}"),
    ))
}

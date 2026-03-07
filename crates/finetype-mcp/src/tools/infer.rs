//! `infer` tool — classify string values into semantic types.

use crate::FineTypeServer;
use rmcp::model::{CallToolResult, Content, ErrorData};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct InferRequest {
    /// One or more string values to classify. For column-mode inference, pass multiple values.
    #[schemars(description = "String values to classify (single value or list for column-mode)")]
    pub values: Vec<String>,

    /// Optional column header name for context-aware classification.
    #[schemars(description = "Column header name for disambiguation (e.g. 'email', 'country')")]
    pub header: Option<String>,
}

pub async fn handle(
    server: &FineTypeServer,
    request: InferRequest,
) -> Result<CallToolResult, ErrorData> {
    if request.values.is_empty() {
        return Err(ErrorData::invalid_params(
            "At least one value is required",
            None,
        ));
    }

    let classifier = server.classifier().read().await;

    if request.values.len() == 1 {
        // Single-value mode: use the underlying ValueClassifier directly
        let value = &request.values[0];
        let result = classifier
            .classifier()
            .classify(value)
            .map_err(|e| ErrorData::internal_error(format!("Inference error: {}", e), None))?;

        // Extract domain from label
        let domain = result
            .label
            .split('.')
            .next()
            .unwrap_or("unknown")
            .to_string();

        let json = serde_json::json!({
            "mode": "value",
            "type": result.label,
            "confidence": result.confidence,
            "domain": domain,
            "top_candidates": result.all_scores.iter().take(5)
                .map(|(label, score)| serde_json::json!({"type": label, "score": score}))
                .collect::<Vec<_>>(),
        });

        let summary = format!(
            "**{}** (confidence: {:.1}%, domain: {})",
            result.label,
            result.confidence * 100.0,
            domain,
        );

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string_pretty(&json).unwrap_or_default()),
            Content::text(summary),
        ]))
    } else {
        // Column mode: use ColumnClassifier for vote aggregation + disambiguation
        let result = if let Some(ref header) = request.header {
            classifier.classify_column_with_header(&request.values, header)
        } else {
            classifier.classify_column(&request.values)
        }
        .map_err(|e| ErrorData::internal_error(format!("Inference error: {}", e), None))?;

        let domain = result
            .label
            .split('.')
            .next()
            .unwrap_or("unknown")
            .to_string();

        // Look up broad_type from taxonomy
        let broad_type = server
            .taxonomy()
            .get(&result.label)
            .map(|d| d.broad_type.clone())
            .unwrap_or_default();

        let json = serde_json::json!({
            "mode": "column",
            "type": result.label,
            "confidence": result.confidence,
            "domain": domain,
            "broad_type": broad_type,
            "samples_used": result.samples_used,
            "is_generic": result.is_generic,
            "detected_locale": result.detected_locale,
            "disambiguation_applied": result.disambiguation_applied,
            "disambiguation_rule": result.disambiguation_rule,
            "vote_distribution": result.vote_distribution.iter().take(10)
                .map(|(label, frac)| serde_json::json!({"type": label, "fraction": frac}))
                .collect::<Vec<_>>(),
        });

        let mut summary = format!(
            "**{}** → `{}` (confidence: {:.1}%, {} samples)",
            request.header.as_deref().unwrap_or("(no header)"),
            result.label,
            result.confidence * 100.0,
            result.samples_used,
        );

        if let Some(ref locale) = result.detected_locale {
            summary.push_str(&format!(" | locale: {}", locale));
        }
        if result.is_generic {
            summary.push_str(" | generic");
        }
        if result.disambiguation_applied {
            if let Some(ref rule) = result.disambiguation_rule {
                summary.push_str(&format!(" | rule: {}", rule));
            }
        }

        Ok(CallToolResult::success(vec![
            Content::text(serde_json::to_string_pretty(&json).unwrap_or_default()),
            Content::text(summary),
        ]))
    }
}

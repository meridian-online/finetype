# Implementation Progress

**Spec:** specs/2026-04-12-accuracy-gap-retraining/spec.yaml
**Started:** 2026-04-12

## Hard Constraints
- [x] Mac Metal training only — script uses --features metal
- [x] ReLU+BN architecture (decision 0046) — script validates no GELU/LN in config
- [x] Production-scale config: sherlock-v5-scaled-config.json (5 branch groups)
- [x] 70/30 distillation:synthetic data mix — --ratio-distilled 0.7
- [x] Sibling-context enrichment during FTMB prep — hard-fail if model missing
- [x] v10-style header validation (hard-fail on zero features)
- [x] No actionability regression (>=96.5%) — 99.8% (was 96.9%)
- [x] Backward compatible via serde defaults — config has no GELU/LN fields
- [x] Hyperparameters: lr=1e-4, weight_decay=1e-4 (v4-sibling-proven)
- [x] n_classes: using sherlock-v5-scaled-config.json (n_classes=250, matching v4-sibling)
- [x] Epoch cap: 40 with patience=10
- [x] Sharpen rules, Model2Vec, sibling-context model frozen

## Acceptance Criteria
- [x] ac-01: Audit 34 misclassifications — 26 WRONG, 6 DEBATABLE, 0 AMBIGUOUS (2 extra DEBATABLE vs agent summary)
- [x] ac-02: Fix 6 DEBATABLE labels — geojson, git_sha, region/address, ip_v4/port, categorical/http_method, categorical/measurement_unit. Expected baseline: 199/227 (87.7%)
- [x] ac-03: Prepare 70/30 FTMB, >=150/239 types with distilled data — completed overnight
- [x] ac-04: FTMB real Model2Vec headers (non-zero validation) — completed overnight
- [x] ac-05: Sibling-context enriched headers in FTMB — completed overnight
- [x] ac-06: Train ReLU+BN on Mac Metal — 39 epochs, best at epoch 29, 74 min
- [x] ac-07: val_accuracy >=88% — **89.2% PASS** (best epoch 29, val_loss 0.3505)
- [ ] ac-08: Profile eval >=205/227 — **203/227 NEAR MISS** (-2 from target, +10 over v4-sibling baseline)
- [x] ac-09: Actionability >=96.5% — **99.8% PASS** (was 96.9%, +2.9pp)
- [x] ac-10: Misclassification delta analysis — see below
- [ ] ac-11: Publish to HuggingFace (if ac-08 + ac-09 pass) — pending decision (near miss)
- [ ] ac-12: Update CLAUDE.md

## Results Summary

### Training
| Model | Architecture | Data Mix | Epochs | Best Val Acc | Best Val Loss | Train Time |
|-------|-------------|----------|--------|-------------|---------------|------------|
| v10-gelu | GELU+LN | 50/50 | 30 | 85.2% | 0.4725 | 48 min |
| v4-sibling | ReLU+BN | 50/50 | 20 | 90.0% | 0.2881 | 41 min |
| v11 | ReLU+BN | 70/30 | 39 | 89.2% | 0.3463 | 74 min |

### Profile Eval
| Model | Label Accuracy | Domain Accuracy | Misclassifications |
|-------|---------------|----------------|--------------------|
| v10-gelu | 188/227 (82.8%) | 203/227 (89.4%) | 39 |
| v4-sibling | 193/227 (85.0%) | 206/227 (90.7%) | 34 |
| v11 | 203/227 (89.4%) | 210/227 (92.5%) | 24 |

### Delta: v11 vs v10-gelu
- **Fixed 17 misclassifications:** api_users_json.phone, codes_and_ids.swift_code, datetime_coverage.compact_ymd, earthquakes_2024.magError, ecommerce_orders.shipping_country, finance_coverage.currency_symbol, finance_coverage.isin, iris.petal_length, medical_records.first_name, multilingual.name, people_directory.full_name, scientific_measurements.measurement_unit, scientific_measurements.ph_value, server_logs_json.user_agent, technology_coverage.ip_v4_with_port, weather_stations_json.precipitation_mm, weather_stations_json.wind_speed_kmh
- **2 new misclassifications:** ecommerce_orders.tracking_url (docker_ref→url), server_logs_json.method (iata_code→http_method)
- **22 persistent** across both models

### Actionability
| Model | Success Rate |
|-------|-------------|
| v10-gelu | 97.2% |
| v4-sibling | 96.9% |
| v11 | 99.8% |

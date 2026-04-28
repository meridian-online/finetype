# v16 N=1 email regression — closed (resolved by v19)

**Status:** closed (resolved by intermediate work)
**Closed:** 2026-04-28
**Resolution:** verified fixed on `models/default → sherlock-v19-relu-s42`

## What was reproduced

Original v16 behaviour (interview, 2026-04-20):

```
$ finetype infer -i "john@example.com" --mode column   # under v16
representation.text.plain_text                         # ❌ regression
```

Re-run under v19 (2026-04-28, on `models/default →
sherlock-v19-relu-s42`):

```
$ finetype infer -i "john@example.com" --mode column
identity.person.email                                  # ✅ fixed
```

## Per-input verification (v19, N=1, --mode column)

```
| Input                       | Expected label                    | Got                                | Confidence |
|-----------------------------|-----------------------------------|------------------------------------|------------|
| john@example.com            | identity.person.email             | identity.person.email              | 0.981      |
| https://example.com/path    | technology.internet.url           | technology.internet.url            | (control)  |
| 192.168.1.1                 | technology.internet.ip_v4         | technology.internet.ip_v4          | (control)  |
```

Email regression resolved at v19; URL and IPv4 controls (which never
regressed under v16) continue to classify correctly.

## Why it resolved without a value-based rule

The interview proposed two fix directions per decision 0048:

1. A value-based sharpen rule (R32?) — snap to email on universal
   regex match.
2. A retraining change (data blend / loss weight) so the model itself
   recovers email at N=1.

v19 took path 2 implicitly. The 5-branch ReLU+BatchNorm architecture
(sherlock-v19-relu-s42, val_acc 0.9173) emits `identity.person.email`
with 0.981 confidence on the N=1 case — a healthy margin, not a
borderline rescue. No new sharpen rule was needed; the model is
robust at N=1 on its own.

This pattern — a regression spec held at interview state, resolved
incidentally by a downstream model promotion — is exactly the
"interview-state rot" failure mode flagged in
`orbit/memos/2026-04-27-n1-email-regression-rot.md`. The remediation
proposed there (verify against current default + add a regression
test) is what this closure ships.

## Regression test added

`crates/finetype-cli/tests/cli_golden.rs` gains three tests under a
new `INFER REGRESSION GUARDS` section:

```
| Test                              | Asserts                                      |
|-----------------------------------|----------------------------------------------|
| golden_infer_n1_email_column      | N=1 email column → identity.person.email     |
| golden_infer_n1_url_column        | N=1 URL column → technology.internet.url     |
| golden_infer_n1_ipv4_column       | N=1 IPv4 column → technology.internet.ip_v4  |
```

All three pass on v19. The email test is the regression guard
proper; URL and IPv4 are belt-and-braces controls so a future N=1
regression in the technology.internet domain trips a test before
it ships.

Run: `cargo test -p finetype-cli --test cli_golden -- --ignored golden_infer_n1`

## Decision

**No spec.yaml. No rule. No retrain.** This artefact closes by
verification + regression test. The model improvement (v19
promotion, MADR 0069) carried the fix.

## Process learning

The interview lived in `orbit/specs/` for 8 days across two model
promotions (v18 held, v19 shipped) without re-verification. The
N=1-regression-rot memo names the failure: interview-state files
older than one model promotion are stale by definition.

Recommended addition to the orbit guidance: any `interview.md`
without a sibling `spec.yaml` after one model promotion gets
triaged — close (resolved), close (won't-fix), or promote to
spec.yaml. None of those is "leave it alone."

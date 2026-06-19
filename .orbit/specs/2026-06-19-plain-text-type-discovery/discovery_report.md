# What's hiding behind plain_text — and what's worth building

**Headline:** Of every 100 columns FineType labels "plain text", roughly
**8 are actually three specific, nameable types it has no word for** —
Windows file paths, email Message-IDs, and dotted code identifiers. The
rest is genuine prose, plus a long tail of messy patterns no analyst would
confidently name. We should build exactly those three.

`plain_text` is FineType's largest single bucket: **447,422 corpus columns**
— the residual it reaches for when nothing tighter fits. That makes it the
richest place to look for types the taxonomy is missing.

## How the three winners were chosen

Two axes, multiplied, then gated on volume:

1. **How common** — measured as *distinct datasets*, not raw column count.
   This matters: raw counts are wildly inflated by replication. `snake_token`
   looked like 28,291 columns but collapsed to 1,709 once you count distinct
   sources (the same Odoo access-rights table copied 16×). `pos_tagged`
   looked like 6,965 and collapsed to 74 (a handful of NLP corpora). Honest
   breadth is the only fair volume signal.

2. **How confidently a blind panel can name it** — three independent expert
   agents named each cluster from values alone, with no header and no
   hypothesis, scoring 0–1 on whether a panel would agree. Unanimous,
   skeptical, and told to flag clusters that lump two types together.

| Rank | Type | Distinct datasets | Panel confidence | Score | Build? |
|---|---|---|---|---|---|
| 1 | **windows_file_path** | 7,651 | 0.97 (unanimous) | 7,421 | ✅ |
| 2 | **email_message_id** | 3,032 | 0.95 (unanimous) | 2,880 | ✅ |
| 3 | **dotted_qualified_name** | 1,677 | 0.79 (unanimous) | 1,325 | ✅ |
| 4 | numeric_range | 1,855 | 0.38 ("mixed" ×3) | 705 | ❌ not one type |
| 5 | quantity+unit | 1,530 | 0.37 ("mixed" ×3) | 566 | ❌ not one type |
| 6 | snake/identifier soup | 1,709 | 0.31 ("mixed" ×3) | 530 | ❌ residual |
| 7 | unix/url path | 178 | 0.77 | 137 | ❌ below volume bar |
| 8 | pos_tagged text | 74 | 0.88 (clean) | 65 | ❌ below volume bar |

**The volume bar (≥1,000 distinct datasets) and the precision principle do
real work here.** They reject two tempting-but-wrong candidates:

- **Clean but rare** — `pos_tagged` text scored 0.88 nameability (a panel
  instantly recognises `Well/UH ,/, um/UH`), but it lives in only 74 source
  datasets. Building a taxonomy type for one NLP annotation format is not
  worth the label-space slot.
- **Common but not actually a type** — `numeric_range`, `quantity+unit`, and
  the `snake_token` soup each clear the volume bar on count, but all three
  panellists independently flagged them "mixed": ranges lumped with
  hyphenated LOINC-style codes, financial magnitudes (`78.78M`) lumped with
  Fahrenheit temperatures (`19 F`), enumerated labels lumped with config
  keys. You cannot write an honest validation for "a range OR a code OR an
  age band" — and a validation that matches everything validates nothing.

## Why the three winners are real, not artefacts

Each pattern was tested on the full 447k for false positives — the precision
test that separates "is this type" from "is not":

- **windows_file_path** — `C:\Windows\System32\drivers\WdfLdr.sys`. 7,913
  columns match; only 87 (1.1%) fire on multi-word text, and those are real
  paths with spaces in folder names. Drive-letter/UNC + backslash is
  unmistakable.
- **email_message_id** — `<30365805.1075860998985.JavaMail.evans@thyme>`. The
  angle-bracket `<left@right>` grammar is unambiguous. Caveat worth naming:
  our corpus sample is Enron-heavy (one provenance), but RFC 2822 Message-ID
  is a universal type every email carries.
- **dotted_qualified_name** — `org.apache.commons.math.fraction.Fraction`.
  6,478 match; **zero** fire on prose. The one boundary to watch is
  hostnames (`www.x.com` is also dotted) — the separating signal is a
  CamelCase / non-TLD final segment, a Sharpen-precedence call for the
  retrain, not the validator.

## What we don't know yet

These three are *built* (definitions + generators authored and precision-
validated) but not yet *live*. FineType's label space is deliberately locked
at 240 in lockstep across three places — the taxonomy, the hand-maintained
category map, and the trained model's output dimension (the test suite
enforces taxonomy ≈ model within ±1). A new type only becomes predictable
after a retrain lifts 240→243 and clears the full promotion gate stack
(gold-anchor → drift proxy → gold corpus → corpus-honest). That retrain is
the next step; this spec hands it three precise, pre-validated targets.

**One line for a stakeholder:** about 8% of FineType's "plain text" columns
are really file paths, email Message-IDs, or code identifiers — three types
worth teaching it; the rest is genuine prose or noise not worth a name.

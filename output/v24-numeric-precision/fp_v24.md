# v24 ac-00 re-baseline — model=models/sherlock-v24-numeric-relu-s42
# n=300/cluster rows=1000 seed=42

## utc->int: datetime.offset.utc (ydf says representation.numeric.integer_number)
   sampled=300 profiled=300 v19_fp=240 v19_fp_rate=0.8000  -> KEEP (v19 still mistypes)
   v19 instead assigns: representation.numeric.integer_number=47, representation.scientific.rna_sequence=6, finance.banking.swift_bic=3, representation.identifier.increment=2

## bool->int: representation.boolean.binary (ydf says representation.numeric.integer_number)
   sampled=298 profiled=298 v19_fp=130 v19_fp_rate=0.4362  -> KEEP (v19 still mistypes)
   v19 instead assigns: representation.numeric.integer_number=167, unknown=1

## url->int: technology.internet.url (ydf says representation.numeric.integer_number)
   sampled=297 profiled=297 v19_fp=297 v19_fp_rate=1.0000  -> KEEP (v19 still mistypes)

## int->dec: representation.numeric.integer_number (ydf says representation.numeric.decimal_number)
   sampled=298 profiled=298 v19_fp=0 v19_fp_rate=0.0000  -> DROP (already ~0 on v19)
   v19 instead assigns: unknown=287, representation.numeric.decimal_number=11

=== JSON ===
[
  {
    "cluster": "utc->int",
    "sense_fp": "datetime.offset.utc",
    "ydf_correct": "representation.numeric.integer_number",
    "sampled": 300,
    "profiled": 300,
    "v19_fp_count": 240,
    "v19_fp_rate": 0.8,
    "verdict": "KEEP (v19 still mistypes)",
    "v19_reassigns_to": [
      {
        "label": "representation.numeric.integer_number",
        "n": 47
      },
      {
        "label": "representation.scientific.rna_sequence",
        "n": 6
      },
      {
        "label": "finance.banking.swift_bic",
        "n": 3
      },
      {
        "label": "representation.identifier.increment",
        "n": 2
      }
    ],
    "extract_err": 0
  },
  {
    "cluster": "bool->int",
    "sense_fp": "representation.boolean.binary",
    "ydf_correct": "representation.numeric.integer_number",
    "sampled": 298,
    "profiled": 298,
    "v19_fp_count": 130,
    "v19_fp_rate": 0.4362,
    "verdict": "KEEP (v19 still mistypes)",
    "v19_reassigns_to": [
      {
        "label": "representation.numeric.integer_number",
        "n": 167
      },
      {
        "label": "unknown",
        "n": 1
      }
    ],
    "extract_err": 2
  },
  {
    "cluster": "url->int",
    "sense_fp": "technology.internet.url",
    "ydf_correct": "representation.numeric.integer_number",
    "sampled": 297,
    "profiled": 297,
    "v19_fp_count": 297,
    "v19_fp_rate": 1.0,
    "verdict": "KEEP (v19 still mistypes)",
    "v19_reassigns_to": [],
    "extract_err": 3
  },
  {
    "cluster": "int->dec",
    "sense_fp": "representation.numeric.integer_number",
    "ydf_correct": "representation.numeric.decimal_number",
    "sampled": 298,
    "profiled": 298,
    "v19_fp_count": 0,
    "v19_fp_rate": 0.0,
    "verdict": "DROP (already ~0 on v19)",
    "v19_reassigns_to": [
      {
        "label": "unknown",
        "n": 287
      },
      {
        "label": "representation.numeric.decimal_number",
        "n": 11
      }
    ],
    "extract_err": 2
  }
]

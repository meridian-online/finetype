# evidence/

The tracked, reviewable record behind every accuracy number this repo quotes. Kilobytes,
not gigabytes: headline reports and the fixture manifest, nothing else.

`output/` stays blanket-ignored — it holds the multi-gigabyte run artefacts (FTMBs,
prediction TSVs, per-column report tables) that regenerate from the scripts. Those are
working material. What belongs here is the small, permanent part: *which ground truth was
this measured on, and what did it come out as.*

## What lives here

| | |
|---|---|
| `fixtures.json` | The fixture manifest. Every gold fixture *version*, content-addressed by sha256, and the baselines recorded on each one. |
| `*-headline.json` | One score record per published measurement — the fixture id, the counts, and the bar it was compared against. Written by `scripts/score_clean_label.sh --evidence-dir evidence`. |
| `*.md` | Headline reports meant to be read. The per-label tables that back them stay in `output/`. |

Nothing that grows without bound. If an artefact is regenerable from a script and a
fixture id, it belongs in `output/`.

## Why a manifest instead of a number in a script

An accuracy bar is a property of a **(model, fixture, binary)** triple. Write one down as
a float in a shell script and it stops being true the first time a gold label is
re-adjudicated — and then it fails every later candidate for the wrong reason. That is not
hypothetical: a `0.853` bar written on 2026-06-28 survived 37 label re-adjudications and
106 added columns, and by 2026-07-25 the *unchanged shipped model* scored 25 columns above
it on a correctly-wired run. The bar had gone stale, not the model.

So: fixture versions are registered by content hash, scores are recorded against a fixture
version, and a comparison is only offered when the bar and the candidate were measured on
the same ground truth. Every score a script prints carries the fixture id beside it.

## Using it

```sh
# which registered version is the checked-out gold corpus?
scripts/evidence.py resolve-fixture --path eval/gold/gold_corpus.tsv

# what is registered, and what has been measured on it?
scripts/evidence.py list

# the manifest is internally consistent (runs in CI)
scripts/evidence.py verify
```

Gold has changed — register the new version before measuring anything on it:

```sh
scripts/evidence.py register-fixture --path eval/gold/gold_corpus.tsv \
    --id gold-YYYY-MM-DD --note "what changed and why"
```

Fixture ids are immutable and one content hash maps to exactly one id. A new gold corpus
is a new fixture version; it never overwrites the old one, because the old scores stay
attributable to the ground truth that produced them.

Record a measurement as a reusable bar:

```sh
scripts/evidence.py record-baseline --fixture gold-YYYY-MM-DD \
    --key "<model>/<pipeline>/<binary>" --correct 819 --scored 931 \
    --model m2v8m-s43 --binary 0.6.53 --pipeline "…" --source "<where this was measured>"
```

`--source` is required by `verify`: an unsourced bar is not evidence.

Score against a bar:

```sh
scripts/score_clean_label.sh models/<candidate> <gold.ftmb> <tag> \
    --baseline "m2v8m-s43/composed-reframe/0.6.53" --evidence-dir evidence
```

If the checked-out gold is not a registered version, the run stops. If the requested bar
was never measured on that version, the run stops. If the bar's denominator differs from
the run's, the run stops. None of those are comparisons worth making, and each of them
used to produce a plausible-looking wrong number instead.

## CI

`scripts/evidence.py verify` and `scripts/test_score_clean_label.sh` both run on every PR.
`verify` fails if a recorded score does not equal its own `correct/scored`, if a bar has no
source, or if a registered fixture file has been edited on disk without a new version being
registered. The harness runs the scorer against stubs and asserts that a failing stage
fails the script rather than falling back to a substitute artefact.

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
| `fixtures.json` | The fixture manifest. Every gold fixture *version*, content-addressed by sha256, the taxonomy version its labels were adjudicated under, and the baselines recorded on each one. |
| `release-<version>.md` | What a release measured at, and on which fixture. **Generated** from `fixtures.json` — see below. |
| `*-headline.json` | One score record per published measurement — the fixture id, the counts, and the bar it was compared against. Written by `scripts/score_clean_label.sh --evidence-dir evidence`. |
| `*.md` | Headline reports meant to be read. The per-label tables that back them stay in `output/`. |

Nothing that grows without bound. If an artefact is regenerable from a script and a
fixture id, it belongs in `output/`.

### The size rule is enforced, not trusted

`scripts/evidence.py verify` fails if any file here exceeds **200 KB**, if the directory
exceeds **1 MB** in total, if a file carries a suffix outside `.md .json .tsv .csv .txt`,
or if a file is not UTF-8 text despite its suffix — a parquet renamed to `.tsv` is still a
parquet. The moment this directory starts holding megabytes the split is in the wrong
place, and the fix is to move artefacts back to `output/` and keep the report, not to
raise the limit.

## The release report is generated, never written

`evidence/release-0.6.53.md` is rendered from `fixtures.json`:

```sh
scripts/evidence.py render-release --binary 0.6.53
scripts/evidence.py render-release --binary 0.6.53 --check   # CI: has it drifted?
```

A number in prose that the manifest does not carry is precisely the transcription error
this directory exists to prevent, so the prose is not allowed to be an independent copy.
`verify` re-renders every release listed under `reports` in the manifest and fails if the
committed file differs by a byte — and fails if the file is missing altogether. That is
what makes "the 0.6.53 baseline is committed" a checked fact rather than a claim.

The renderer will not subtract two scores measured on different fixture versions. It lists
them as **refused comparisons** instead, with both numbers and both fixture ids, so the
reader can see the pair without being handed a difference that measures nothing.

## Taxonomy versions

The taxonomy is not a numbered artefact — it is the seven tracked
`labels/definitions_*.yaml` files, compiled into the binary by `include_str!`. So its
version is its content:

```sh
scripts/evidence.py taxonomy-version              # tax-<12 hex>
scripts/evidence.py taxonomy-version --format json
```

The id is derived from the digest, so there is no number anyone can forget to bump or type
in wrong — `verify` rejects a recorded `taxonomy.version` that is not the first twelve hex
of its own `taxonomy.sha256`.

Each fixture records the taxonomy version its labels were **adjudicated** under, read from
the commit in its `taxonomy.commit`. That is a historical fact and it never expires: a
fixture is not invalidated because a type was added elsewhere later. What *is* fatal is a
fixture whose ground truth names a label the checked-out taxonomy no longer defines —
a score against a type that does not exist cannot be attributed to anything — so `verify`
checks every distinct label in the on-disk fixture against the current vocabulary and
reports the drift between the two versions as a note.

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
    --id gold-YYYY-MM-DD --label-column curated_label --note "what changed and why"
```

`--label-column` is what lets `verify` check the fixture's labels against the taxonomy;
without it the recorded taxonomy version is decorative. Registering a *historical*
version whose bytes are only in git takes the hash directly, and reads that commit's
vocabulary rather than today's:

```sh
git show <rev>:labels/definitions_X.yaml > /tmp/at-rev/labels/definitions_X.yaml   # ×7
scripts/evidence.py register-fixture --id gold-YYYY-MM-DD \
    --sha256 <hash> --rows <n> --record-path eval/gold/gold_corpus.tsv \
    --taxonomy-root /tmp/at-rev --taxonomy-commit <rev> --note "…"
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

`scripts/evidence.py verify`, `scripts/test_evidence.sh` and
`scripts/test_score_clean_label.sh` all run on every PR.

`verify` fails if a recorded score does not equal its own `correct/scored`, if a bar has no
source, if a registered fixture file has been edited on disk without a new version being
registered, if a fixture does not name the taxonomy version its labels were adjudicated
under or names one not derived from its own digest, if a fixture asserts a label the
taxonomy no longer defines, if `evidence/` has taken on a non-text artefact or grown past
its budget, or if a committed release report has drifted from the manifest.

`test_evidence.sh` drives all of that against a sandbox repo root — its own taxonomy, gold
fixture and evidence directory — so each rejection is attributable to the thing its case
names. It asserts the sandbox is *accepted* before it breaks anything; without that, every
case below it would pass for free.

`test_score_clean_label.sh` runs the scorer against stubs and asserts that a failing stage
fails the script rather than falling back to a substitute artefact.

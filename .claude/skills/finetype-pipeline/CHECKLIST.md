# FineType Pipeline — Manual Verification Checklist

This checklist verifies the `/finetype-pipeline` skill walks an agent through
the complete v0.6.19 three-step pipeline on a representative CSV, not just
profile and stop.

**Pipeline under test (post-MADR-0070/0071):**

```
profile → profile -o json-schema → validate --db --table
```

There is no `load` verb. The typed-output path lives on `validate --db --table`.

---

## Pre-flight

- [ ] Pick a representative CSV with mixed column types (text, dates, numbers,
      identifiers). The repo's `tests/fixtures/contacts.csv` works, or any
      CSV in `corpora/eval/`.
- [ ] Confirm the binary is at v0.6.19 or higher: `finetype --version`.
- [ ] Confirm `duckdb` is on `PATH` (required for step 3 materialisation):
      `which duckdb`.

## Step 1 — Profile (must run)

- [ ] Agent invokes `finetype profile -f <CSV>` and reads the output (TYPE,
      BROAD, CONF columns surface).
- [ ] Agent does **not** stop here. The skill explicitly warns "Profile is
      step 1, not the destination" — verify the agent treats it as such.

## Step 2 — JSON Schema (must run)

- [ ] Agent invokes `finetype profile -f <CSV> -o json-schema > schema.json`
      and saves the schema.
- [ ] Agent does **not** invoke the retired `finetype schema <CSV>` form.
      Table-mode JSON Schema export lives on `profile -o json-schema` since
      v0.6.19 (MADR 0070).

## Step 3 — Validate + Materialise (must run)

- [ ] Agent invokes `finetype validate <CSV> schema.json --db <out.db> --table <name>`.
- [ ] Agent does **not** invoke `finetype load …`. That verb was removed
      in v0.6.19 (MADR 0071) and now errors via clap's unknown-subcommand
      handler with exit code 2.
- [ ] Agent reads the validation grade and reject sidecar
      (`finetype_reject_errors`) if any rows fail.

## Negative checks

- [ ] Agent does **not** stop after step 1 (profile-only).
- [ ] Agent does **not** treat `validate` (without `--db --table`) as the
      terminal step when typed materialisation is the goal. Check-only
      validation is a quality gate, not a typed-output path.
- [ ] Agent does **not** present `load` as a current pipeline step in any
      explanation it produces.

## Pass criteria

The skill **passes** this checklist when an agent, given a representative CSV
and asked to load it into DuckDB, invokes all three commands in sequence
without prompting and without referencing the removed `load` verb.

The skill **fails** this checklist if the agent stops after profile, skips
the JSON Schema step, or attempts to invoke `finetype load`.

---

**Maintenance:** when the pipeline surface changes (new step, removed verb,
new flag), update this checklist alongside `SKILL.md`. Drift between
`SKILL.md` and the binary surface is caught by
`scripts/verify-cli-skill-coverage.sh` for the CLI skill; this checklist is
the corresponding artefact for the pipeline skill's behavioural claim.

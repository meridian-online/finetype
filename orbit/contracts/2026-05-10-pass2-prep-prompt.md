# Pass-2 tabletop brief preparation prompt

This file is the prompt for the agent that prepares the pass-2 tabletop
brief on or around 2026-05-10. The pass-1 tabletop (2026-05-03) tried to
schedule this via `CronCreate` but `durable: true` was silently ignored
by the harness — session-only crons cannot survive 7 days of REPL
restarts.

## Invocation options

**A. Manual** — paste the prompt below into a fresh Claude Code session
in `~/github/meridian-online/finetype/`:

```bash
cd ~/github/meridian-online/finetype/
claude  # then paste the "Prompt" section below
```

**B. Headless one-shot** — single invocation:

```bash
cd ~/github/meridian-online/finetype/
claude -p "$(awk '/^## Prompt/,0' orbit/contracts/2026-05-10-pass2-prep-prompt.md | tail -n +2)"
```

**C. launchd one-shot** — Carson task. Add a one-shot LaunchAgent that
fires on 2026-05-10 at 06:03 Australia/Brisbane invoking option B's
command. Tracked via finetype-53r (launchd plist for cron-firing agent)
— ideally the same plist gains a `StartCalendarInterval` entry for the
2026-05-10 06:03 AEST one-shot.

Hugh's call which to use; option A is the zero-infrastructure fallback.

## Prompt

Pass-2 tabletop brief preparation for FineType GitTables 90% round-trip contract.

This was scheduled on 2026-05-03 by the pass-1 tabletop session (Nightingale + Hugh). Today is 2026-05-10 — pass-2 is happening this week and you are preparing the brief that loads the next session.

Working directory: ~/github/meridian-online/finetype/

## Inputs

- Contract: orbit/contracts/2026-05-03-gittables-90-percent-roundtrip.yaml — read this first; brief structures around its 5 sections.
- Load-bearing registry: orbit/contracts/load-bearing-paths.yaml — referenced by §3 B07.
- Cycle logs (may or may not exist yet):
  - eval/gittables/failure_log.tsv (cols documented in finetype-87j bead)
  - eval/gittables/working_slice_coverage.tsv
  - eval/gittables/cycle_log.* (format TBD)
- Bead state: .beads/issues.jsonl. Run `bd show <id>` for these 5 P1 cron-activation beads:
  - finetype-e6d (gate harness)
  - finetype-s16 (content-hash dedup)
  - finetype-nms (cron lockfile + preamble)
  - finetype-87j (append-only logs)
  - finetype-53r (launchd plist — Carson handoff)

## Output

Write orbit/contracts/2026-05-10-pass2-tabletop-brief.md. Structure by the contract's 5 sections; per section provide:

§1 Objective function — gate metric trajectory across cycles, corpus value metric (cumulative coverage % + observed pass rate), proximity to 90% target, recommended amendments to numeric targets if data warrants.

§2 Standing orders — which fired correctly, which got skipped or errored, recommended amendments.

§3 Branch table — which of B01-B09 fired and counts; which never fired (dead code signal or missing scenarios); recommended new branches from unmatched-state observations.

§4 Halts — which of H01-H13 fired with frequency + resolution; which never fired (over-engineered signal); candidate new halts from premortem applying the load-bearing test ("what failure mode does this halt catch that the branch table cannot?").

§5 Escalations — which of E01-E06 raised with ask content captured; recommended amendments.

Pre-activation status (always present): bead status from bd for the 5 P1 beads — which shipped, which open, which blocked. If all 5 still open, brief reframes from "cycle audit" to "barriers to cron activation."

Hot-wash candidates: anything unusual that doesn't fit above; methodology-spine questions surfaced by real cycle data.

## Edge case: no cycle data

If eval/gittables/ is empty or missing, cron has not activated. Brief skips cycle-data sections; central content becomes the activation gap with specific unblocking actions per bead.

## Write discipline

The brief is the AAR for the pass-2 session that hasn't happened yet — it loads the session, doesn't conclude it. Frame as "evidence + recommendations for human review," not "decisions." Lead with single recommendations per Hugh's working conventions (lead-with-single-recommendation rule). Reserve 2-3 option menus only for true architectural forks. Numeric: cite specific cycles, halt IDs, line counts. Brief is data, not narrative.

## Action

Commit + push directly to main (no PR — per the contract's no-PR-merge policy; this is contract-mandated work, not arbitrary code change):

  Pass-2 tabletop brief — gittables 90% round-trip contract

  Auto-prepared 2026-05-10 from cycle logs + bead state. Loads the pass-2
  tabletop session per orbit/contracts/2026-05-03-gittables-90-percent-
  roundtrip.yaml provenance.next_tabletop spec.

DO NOT run the tabletop itself — that is interactive with Hugh. Your output is the brief; he runs the session.

Notify Hugh once the brief is committed + pushed.

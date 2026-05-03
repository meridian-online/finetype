# launchd setup — cron-firing FineType agent

This document is the self-contained infrastructure handoff for setting up
the macOS launchd plist that fires the autonomous FineType agent every
2h, per the active autonomy contract at
`orbit/contracts/2026-05-03-gittables-90-percent-roundtrip.yaml`.

It exists because the contract requires the cron-firing agent to run
unattended on macOS, and `CronCreate` is REPL-bound (dies with the
Claude Code session). launchd is the right mechanism for unattended
recurring + one-shot triggers.

Tracked via bead `finetype-53r`.

## Why launchd, not cron

- **Survives reboots** — launchd reloads agents on user login.
- **Survives REPL closure** — runs as a system-managed process, not a
  child of any open Claude Code session.
- **One mechanism for both recurring and one-shot** — the same plist
  can specify `StartInterval` (recurring) AND `StartCalendarInterval`
  (one or more pinned times).
- **Native macOS** — no third-party scheduler required.

## Required behaviour

### Recurring (every 2h)

The contract's §1 cycle cadence specifies measurement-mode cycles every
2h. Each fire invokes `claude -p "<prompt>"` with the cycle prompt and
the FineType repo as the working directory. The agent reads the
contract, runs the cycle's standing orders, branches, halts, and
escalation surfaces, then exits.

### One-shot (pass-2 tabletop brief preparation)

A pinned trigger at `2026-05-09T20:03:00Z` (Sun May 10 06:03 AEST)
fires the pass-2 brief preparation prompt at
`orbit/contracts/2026-05-10-pass2-prep-prompt.md`. Same launchd plist;
either an additional `StartCalendarInterval` entry on the recurring
plist, or a sibling plist file. Sibling plist is cleaner (separation of
concerns) but either works.

## Plist specification

Suggested canonical path:
`~/Library/LaunchAgents/online.meridian.finetype-cron.plist`

Required keys:

| Key | Value | Notes |
|---|---|---|
| `Label` | `online.meridian.finetype-cron` | Reverse-DNS, matches filename |
| `ProgramArguments` | array | `["/usr/local/bin/bash", "-lc", "<invocation>"]` to pick up user PATH; full path to `claude` if PATH isn't reliable under launchd |
| `WorkingDirectory` | `/Users/hugh/github/meridian-online/finetype` | Cron prompt assumes this CWD |
| `StartInterval` | `7200` | 2h in seconds |
| `EnvironmentVariables` | dict | At minimum: `PATH` including `/opt/homebrew/bin`, `/Users/hugh/.cargo/bin`, locations of `finetype` + `duckdb` + `bd` binaries; `HOME=/Users/hugh` |
| `StandardOutPath` | `/Users/hugh/Library/Logs/finetype-cron/stdout.log` | Path must exist |
| `StandardErrorPath` | `/Users/hugh/Library/Logs/finetype-cron/stderr.log` | Path must exist |
| `RunAtLoad` | `false` | Don't fire immediately on `launchctl load` — let the next 2h boundary catch it |

For the pass-2 one-shot, add (or use a sibling plist):

| Key | Value |
|---|---|
| `StartCalendarInterval` | `{Year: 2026, Month: 5, Day: 10, Hour: 6, Minute: 3}` (Brisbane local time — launchd uses local timezone) |

The cron prompt's location: TBD — `finetype-nms` (cron preamble script)
ships the entry-point shell script that the launchd plist invokes. Until
that lands, the launchd plist can be authored against a placeholder
invocation; final wiring closes once `finetype-nms` ships.

## Lockfile coordination

Per contract halt H04 (`concurrent_cycle`), the agent uses
`/tmp/finetype-cron.lock` to prevent overlapping cycles. launchd's
default behaviour fires the next cycle even if the previous one is
still running — so the lockfile is the structural protection. The
`finetype-nms` preamble script implements the lockfile acquire/release;
launchd just fires the script and trusts the script to handle overlap.

## Logging

`~/Library/Logs/finetype-cron/` holds stdout + stderr per the plist.
Log rotation requirement (ac-05): rotate daily, archive not delete (the
contract's failure analysis may need to walk back through historical
cycle logs). `newsyslog` is the macOS-native rotator —
`/etc/newsyslog.d/finetype-cron.conf` with a config that archives to
`.log.YYYYMMDD.gz` files.

## Disable workflow

For emergency stop or maintenance:

```bash
launchctl unload ~/Library/LaunchAgents/online.meridian.finetype-cron.plist
```

To re-enable:

```bash
launchctl load ~/Library/LaunchAgents/online.meridian.finetype-cron.plist
```

For permanent removal: `launchctl unload` + delete the plist file +
`rm -f /tmp/finetype-cron.lock` (clean up any leftover lock).

## Acceptance criteria

The bead `finetype-53r` lists 5 ACs:

1. launchd plist file shipped at canonical path; loadable via `launchctl`
2. dry-run invocation completes a full cycle (preamble + measurement
   + postamble) end-to-end without any REPL open
3. lockfile interaction prevents concurrent firing — pre-acquire lock,
   verify the launchd-fired cycle skips and logs H04
4. disable + re-enable workflow documented (this file covers it; verify
   the commands work)
5. log rotation at `~/Library/Logs/finetype-cron/` functional; old logs
   archived not deleted

## Coordination with other cron-activation work

Five P1 beads gate cron activation. Work order:

1. `finetype-e6d` — gate-metric harness (so cycles produce measurement value)
2. `finetype-s16` — content-hash dedup (holdout selection depends on it)
3. `finetype-87j` — append-only logs (cycle data has somewhere to land)
4. `finetype-nms` — cron preamble script (the script launchd invokes)
5. `finetype-53r` — this work; depends on `finetype-nms` having shipped first

## Bonus — pass-2 one-shot

If wiring the recurring plist anyway, add the one-shot at the same time:

- Sibling plist file: `online.meridian.finetype-pass2-2026-05-10.plist`
- `StartCalendarInterval` pinned to 2026-05-10 06:03 AEST
- `RunAtLoad: false`
- `Program` invokes `claude -p` with the prompt body extracted from
  `orbit/contracts/2026-05-10-pass2-prep-prompt.md` (option B's
  invocation in that file shows the awk extraction)
- After firing, the plist self-removes (pinned to a single date — won't
  fire again, but cleanliness suggests `rm` on the plist after the
  run completes)

The one-shot is genuinely no-rush — Hugh can manually invoke option A
(paste the prompt) on May 10 if launchd isn't ready by then.

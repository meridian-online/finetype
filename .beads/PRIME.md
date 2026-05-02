# Orbit Execution Context

## Start

```bash
bd ready --type task   # Claimable work (use bd ready unfiltered during planning)
bd memories            # Prior decisions and context
```

## Work Loop

```bash
bd show <id>                # Read acceptance criteria before starting
bd update <id> --claim      # Claim (atomic, prevents races)
# ... implement against acceptance criteria ...
bd update <id> --acceptance "$(new_checked_state)"  # Check off ACs
bd close <id> --reason "what shipped"               # Complete
```

## Decisions

```bash
bd remember "insight"       # Persists across sessions
bd memories <keyword>       # Search prior decisions
```

## Session Close

Before finishing: `bd close <id1> <id2> ...` for all completed work.

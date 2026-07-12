# Roadmap

> `docs/lld/NNN-*.md` filenames referenced below are historical: each LLD is
> deleted once its story ships (see `docs/lld/TEMPLATE.md`), so these are
> provenance notes, not live links.

## Commands

| Command       | Notes |
| ------------- | ----- |
| `init`        | Done  |
| `new`         | Done  |
| `daily`       | Done  |
| `move`        | Done  |
| `archive`     | Done  |
| `list`        | Done  |
| `status`      | Done  |
| `review`      | Done  |
| `config`      | Done  |
| `completions` | Done  |

## Outstanding stories

- [skill.md](user-stories/skill.md) — bundled, installable Claude Code Skill so an agent can drive `ishi` non-interactively to manage a PARA vault. No CLI changes; packaging and documentation only.
- [agent-ergonomics-map.md](user-stories/agent-ergonomics-map.md) — story map for CLI-level agent ergonomics gaps (see `docs/agent-ergonomics.md`), sequencing the stories below:
  - [review.md](user-stories/review.md) Story 004 — `ishi review <item> --keep|--archive|--skip`
  - [exit-codes.md](user-stories/exit-codes.md) Story 001 — semantic exit codes for not-found/invalid-state/invalid-config

Implementation notes for finished work (design rationale, which LLD each
story shipped under) live in git history and in `docs/design.md`, not
here.

## Explicitly out of scope for this pass

Anything not in the README's command table (e.g. sync, plugins, multi-user
config) — not hinted at anywhere in the spec, so not roadmapped until
there's a concrete story for it.

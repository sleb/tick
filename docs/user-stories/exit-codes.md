# User Stories: Semantic exit codes

Not a single subcommand — this covers `main`'s error handling
(`src/main.rs:292` returns `anyhow::Result<()>`), which currently maps every
failure to exit code 1 with an `Error: ...` string on stderr. An agent
running `ishi` can't tell "item not found" from "already archived" from
"invalid config TOML" without string-matching that message, so it can't
decide whether to retry, treat the failure as a no-op, or surface it to the
user. See [../agent-ergonomics.md](../agent-ergonomics.md) gap 3.

The human-readable `Error: ...` message stays exactly as-is for every
failure — this is additive. The exit code becomes a second, machine-checkable
signal alongside the message a human already reads.

---

## User Story 001

- **Summary:** Distinct exit codes for distinct failure categories, with the existing error message unchanged
- **Depends on:** None (cuts across every command's existing error paths)

### Use Case

- **As an** agent driving Ishi on a user's behalf
- **I want to** branch on `ishi`'s exit code to tell "item not found," "item already in an invalid state for this operation," and "invalid config" apart
- **so that** I can decide whether to retry, treat the failure as a no-op, or surface it to the user, instead of parsing stderr text with no stability guarantee

### Acceptance Criteria

- **Scenario:** Item-not-found errors exit with a dedicated code
- **Given:** I am inside an initialized PARA system with no item named `nonexistent`
- **When:** I run `ishi move nonexistent area`
- **Then:** Ishi exits with a dedicated "not found" exit code (distinct from the generic failure code) and still prints `Error: ...` naming `nonexistent` on stderr

- **Scenario:** An operation invalid for the item's current state exits with a dedicated code
- **Given:** I am inside an initialized PARA system with `old-project` already in the Archive
- **When:** I run `ishi archive old-project`
- **Then:** Ishi exits with a dedicated "invalid state" exit code, distinct from both the "not found" code and the generic failure code, and still prints an `Error: ...` explaining `old-project` is already archived

- **Scenario:** An invalid config file exits with a dedicated code
- **Given:** `./.ishi.toml` contains malformed TOML
- **When:** I run any Ishi command that reads config (e.g. `ishi list project`)
- **Then:** Ishi exits with a dedicated "invalid config" exit code and still prints an `Error: ...` describing the TOML parse failure

- **Scenario:** Success still exits 0
- **Given:** I am inside an initialized PARA system with a project `website-redesign`
- **When:** I run `ishi list project`
- **Then:** Ishi exits with code 0, unchanged from current behavior

- **Scenario:** An uncategorized failure falls back to the existing generic code
- **Given:** a failure occurs that doesn't fall into any of the categories above
- **When:** the command exits
- **Then:** Ishi exits 1 with `Error: ...` on stderr, exactly as it does today — the new codes are additive, not a breaking renumbering of every existing failure

- **Scenario:** Exit codes are documented and stable
- **Given:** an agent wants to write logic that branches on Ishi's exit codes
- **When:** it consults Ishi's documentation
- **Then:** each dedicated exit code's meaning is documented (e.g. in the README) as part of the command's contract, so an agent can rely on the number without re-deriving it from source

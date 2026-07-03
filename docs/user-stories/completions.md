# User Stories: `tk completions`

## User Story 001

- **Summary:** Get shell completions for `tk` without writing them by hand
- **Depends on:** None

### Use Case

- **As a** Tick user who wants tab-completion for `tk` subcommands and flags
- **I want to** run `tk completions <shell>`
- **so that** I can install a generated completion script for my shell instead of writing one myself

### Acceptance Criteria

- **Scenario:** Generating a bash completion script
- **Given:** I am using bash
- **When:** I run `tk completions bash`
- **Then:** Tick prints a bash completion script for `tk` to stdout
- **and Then:** nothing is written to disk — I choose where to save it (e.g. `tk completions bash > ~/.local/share/bash-completion/completions/tk`)

- **Scenario:** Generating a zsh completion script
- **Given:** I am using zsh
- **When:** I run `tk completions zsh`
- **Then:** Tick prints a zsh completion script for `tk` to stdout

- **Scenario:** Generating a fish completion script
- **Given:** I am using fish
- **When:** I run `tk completions fish`
- **Then:** Tick prints a fish completion script for `tk` to stdout

- **Scenario:** Generating a PowerShell completion script
- **Given:** I am using PowerShell
- **When:** I run `tk completions powershell`
- **Then:** Tick prints a PowerShell completion script for `tk` to stdout

---

## User Story 002

- **Summary:** Get a clear error instead of a broken script for an unsupported shell
- **Depends on:** Story 001 (the generation flow this validates input for)

### Use Case

- **As a** Tick user who mistypes or guesses at a shell name
- **I want to** be told my shell isn't supported
- **so that** I don't mistake a usage error for a valid (but wrong) completion script

### Acceptance Criteria

- **Scenario:** Unrecognized shell name
- **Given:** I run `tk completions tcsh` (a shell Tick doesn't generate completions for)
- **When:** the command runs
- **Then:** Tick exits with an error naming the shells it does support (`bash`, `zsh`, `fish`, `powershell`)
- **and Then:** nothing is printed to stdout that could be mistaken for a completion script

- **Scenario:** Missing shell argument
- **Given:** I run `tk completions` with no `<shell>` argument
- **When:** the command runs
- **Then:** Tick exits with a usage error indicating `<shell>` is required, rather than guessing a default shell

---

## User Story 003

- **Summary:** Stay current with `tk`'s commands as they change
- **Depends on:** Story 001 (script generation), [init.md](init.md) Story 001, [new.md](new.md) Story 002, [daily.md](daily.md) Story 001, [mv.md](mv.md) Story 001, [list.md](list.md) Story 001, [status.md](status.md) Story 001, [review.md](review.md) Story 001, [config.md](config.md) Story 001 (every top-level command the script must cover)

### Use Case

- **As a** Tick user who has installed a completion script
- **I want to** the script to reflect this installed version of `tk`'s actual commands, subcommands, and flags
- **so that** completions don't drift out of sync with what `tk` really accepts

### Acceptance Criteria

- **Scenario:** Completions cover all top-level commands
- **Given:** I run `tk completions` for any supported shell
- **When:** Tick generates the script
- **Then:** the script includes completions for every top-level command in `tk`'s CLI definition (`init`, `new`, `daily`, `mv`, `list`, `status`, `review`, `config`, `completions`), generated from that definition rather than a hand-maintained list
- **and Then:** running `tk completions` again after a command or flag is added or removed (in a newer `tk` version) reflects that change automatically, with no separate update to the completion script's source

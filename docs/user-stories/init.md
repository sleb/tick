# User Stories: `tk init`

## User Story 001

- **Summary:** Turn the current directory into a PARA system with one command
- **Status:** Completed
- **Depends on:** None

### Use Case

- **As a** new Tick user starting from an empty (or existing) working directory
- **I want to** run `tk init` with no arguments
- **so that** I get a ready-to-use PARA structure right here, without deciding on a project name first

### Acceptance Criteria

- **Scenario:** Initialize in the current directory
- **Given:** I am in a directory that is not already a PARA system
- **When:** I run `tk init`
- **Then:** Tick creates `0-Inbox`, `1-Projects`, `2-Areas`, `3-Resources`, and `4-Archive` in the current directory
- **and Then:** Tick prints `Created PARA system in .`

---

## User Story 002

- **Summary:** Start a new, separately-named PARA system without leaving my current folder
- **Status:** Completed
- **Depends on:** Story 001 (same scaffolding behavior, applied to a named subdirectory)

### Use Case

- **As a** Tick user setting up a new PARA system alongside other work
- **I want to** run `tk init <name>`
- **so that** the new system is scaffolded into its own subdirectory, instead of taking over the directory I'm already in

### Acceptance Criteria

- **Scenario:** Initialize into a named subdirectory
- **Given:** I am in a directory that does not contain a subdirectory called `<name>`
- **When:** I run `tk init my-para`
- **Then:** Tick creates `./my-para` containing `0-Inbox`, `1-Projects`, `2-Areas`, `3-Resources`, and `4-Archive`
- **and Then:** Tick prints `Created PARA system in ./my-para`

---

## User Story 003

- **Summary:** Re-running `init` fills in whatever's missing instead of failing outright
- **Status:** Completed
- **Depends on:** Story 001, Story 002 (fills in gaps for either the current-directory or named-subdirectory target)

### Use Case

- **As a** Tick user who might run `init` more than once, or who deleted a category folder by accident
- **I want to** have `init` create only the category folders that don't already exist
- **so that** I can repair or complete a partial PARA system without it complaining or duplicating what's already there

### Acceptance Criteria

- **Scenario:** Re-initializing a complete PARA system is a no-op
- **Given:** the target directory (current directory, or `./<name>` if given) already contains all five category folders
- **When:** I run `tk init` (with or without a name)
- **Then:** Tick creates no new files or directories
- **and Then:** Tick reports that the PARA system is already complete, with no changes made

- **Scenario:** Re-initializing a partial PARA system fills in the gaps
- **Given:** the target directory contains some but not all of the five category folders (e.g. `0-Inbox` exists but `1-Projects` does not)
- **When:** I run `tk init` (with or without a name)
- **Then:** Tick creates only the missing category folders, leaving existing ones (and their contents) untouched
- **and Then:** Tick reports which folders it created

---

## User Story 004

- **Summary:** Get a clear error instead of a confusing filesystem failure when the target path is unusable
- **Status:** Completed
- **Depends on:** Story 002 (named target), Story 003 (partial-directory handling that this story's error path is an exception to)

### Use Case

- **As a** Tick user who might typo or reuse a name that collides with an existing file
- **I want to** be told when `<name>` already exists as a regular file
- **so that** I understand why `init` didn't succeed instead of seeing a raw filesystem error

### Acceptance Criteria

- **Scenario:** Target name collides with an existing file
- **Given:** `./<name>` already exists but is a regular file, not a directory
- **When:** I run `tk init <name>`
- **Then:** Tick prints an error explaining that `./<name>` already exists and isn't a directory
- **and Then:** no files or directories are created or modified

- **Scenario:** Target name collides with an existing directory that has unrelated contents
- **Given:** `./<name>` already exists as a directory containing files or folders that aren't among the five category folders
- **When:** I run `tk init <name>`
- **Then:** Tick treats it the same as any other existing directory: it creates whichever of the five category folders are missing, and leaves the unrelated contents untouched (see Story 003)
- **and Then:** Tick does **not** treat the unrelated contents as an error

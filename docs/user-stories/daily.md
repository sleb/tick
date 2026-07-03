# User Stories: `tk daily`

`tk daily` is sugar for `tk new --daily` (see [new.md](new.md) story 013) — a
single command for today's note in the Inbox, without having to know or type
today's date. See gap 1 in [spec-gaps.md](../spec-gaps.md) for the ambiguities
these stories resolve.

## User Story 001

- **Summary:** Running `tk daily` for the first time today creates the note
- **Depends on:** [new.md](new.md) Story 002 (named, non-interactive note creation), Story 007 (template rendering)

### Use Case

- **As a** Tick user starting my day
- **I want to** run `tk daily` and get a fresh note named for today
- **so that** I don't have to remember the date format or type it myself

### Acceptance Criteria

- **Scenario:** First run of the day creates the note non-interactively
- **Given:** I am inside an initialized PARA system with the default `daily` template
- **and Given:** no note for today's date exists yet in `0-Inbox`
- **When:** I run `tk daily`
- **Then:** a file named for today's date (e.g. `2026-07-02.md`) is created in `0-Inbox` with the `daily` template rendered into it, `{{title}}` and `{{date}}` filled in with today's date
- **and Then:** Tick prints the path it created, the same as any other non-interactive `tk new` capture
- **and Then:** `$EDITOR` is not invoked — creation is non-interactive, same as `tk new <filename>`

---

## User Story 002

- **Summary:** Running `tk daily` again the same day opens the existing note instead of recreating it
- **Depends on:** Story 001 (note must already exist to be reopened)

### Use Case

- **As a** Tick user returning to my daily note later in the day
- **I want to** run `tk daily` again and pick up where I left off
- **so that** I don't lose earlier notes or get a second, conflicting file for the same day

### Acceptance Criteria

- **Scenario:** Second run of the day opens the existing note in `$EDITOR`
- **Given:** I am inside an initialized PARA system with the default `daily` template
- **and Given:** today's daily note already exists in `0-Inbox`, with content I added earlier
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk daily` again
- **Then:** Tick opens the existing file directly in `$EDITOR`, unchanged from what I last saved — the template is not re-rendered and none of my earlier content is touched
- **and Then:** Tick does not print a `Created ...` line, since no file was created

- **Scenario:** Reopening requires `$EDITOR` to be set
- **Given:** I am inside an initialized PARA system
- **and Given:** today's daily note already exists in `0-Inbox`
- **and Given:** my `$EDITOR` environment variable is not set
- **When:** I run `tk daily`
- **Then:** Tick reports an error that `$EDITOR` must be set to reopen an existing note, rather than silently doing nothing or recreating the file

---

## User Story 003

- **Summary:** The daily note's filename always comes from the current date, never user input
- **Depends on:** Story 001, Story 002 (creation and reopening both rely on this naming rule)

### Use Case

- **As a** Tick user relying on `tk daily` for a consistent daily-note habit
- **I want to** have the filename generated the same way every time
- **so that** I always land on the same note for a given day, whether I'm creating it or reopening it

### Acceptance Criteria

- **Scenario:** Filename uses the configured default extension
- **Given:** I am inside an initialized PARA system with the default `daily` template and default `.md` extension
- **When:** I run `tk daily` on 2026-07-02
- **Then:** the file created is `0-Inbox/2026-07-02.md`

- **Scenario:** `tk daily` takes no filename argument
- **Given:** I am inside an initialized PARA system
- **When:** I run `tk daily some-name`
- **Then:** Tick rejects the command with an error — `tk daily` doesn't accept a filename argument, since the name is always today's date

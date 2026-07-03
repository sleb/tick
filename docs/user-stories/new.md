# User Stories: `tk new`

## User Story 001

- **Summary:** Capture a quick thought without leaving the terminal or naming a file
- **Status:** Needs rework — spec changed 2026-07-02 (editor now opens
  pre-populated with the rendered `note` template instead of a blank
  scratch file, per story 007). The current implementation still does
  simple first-line inference against a blank file; see roadmap.md item 2.

### Use Case

- **As a** Tick user with a fleeting idea
- **I want to** run `tk new` with no arguments and write directly in my editor
- **so that** I can capture the thought immediately without deciding on a filename or category first

### Acceptance Criteria

- **Scenario:** Accept the inferred filename
- **Given:** I am inside an initialized PARA system with the default `note` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new` with no arguments, and my editor opens pre-populated with the rendered `note` template, cursor positioned at the `{{cursor}}` mark on the title line
- **and When:** I type `Website Improvement Ideas` at the cursor (so the title line reads `# Website Improvement Ideas`), save, and exit the editor
- **Then:** Tick prompts `Create "website-improvement-ideas.md"?` with the inferred name pre-filled
- **and Then:** if I accept the prompt, the file is created in `0-Inbox` under that name, containing the frontmatter and heading exactly as I left them, and Tick prints the path it created

- **Scenario:** Override the inferred filename
- **Given:** I am inside an initialized PARA system with the default `note` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new` with no arguments, fill in the pre-populated template, save, exit the editor, and am shown the inferred filename prompt
- **Then:** I can type a different filename instead of accepting the suggestion
- **and Then:** the file is created in `0-Inbox` under the name I typed, and Tick prints the path it created

- **Scenario:** Heading found further down the note is still used
- **Given:** I am inside an initialized PARA system with the default `note` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new`, leave the pre-populated title line blank, write some body text, then add a heading line (e.g. `# Actual Title`) further down, save, and exit the editor
- **Then:** Tick infers the title from that heading line, wherever it falls in the file — not just the line immediately after the frontmatter

- **Scenario:** No heading present falls back to the first line of content
- **Given:** I am inside an initialized PARA system with the default `note` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new`, delete the template's heading line entirely, write plain body text with no `#` heading anywhere, save, and exit the editor
- **Then:** Tick infers the title verbatim from the first non-blank line after the frontmatter block, since no heading line was found

- **Scenario:** Leaving the template unmodified falls back to a timestamp
- **Given:** I am inside an initialized PARA system with the default `note` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new`, save the file exactly as it was pre-populated (no title typed in, no other edits), and exit the editor
- **Then:** Tick prompts with a filename generated from the current timestamp instead of a note title, since the template alone has no title content to infer from

- **Scenario:** Emptying the file falls back to a timestamp
- **Given:** I am inside an initialized PARA system with the default `note` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new`, delete everything the editor was pre-populated with (including the frontmatter), save an empty file, and exit the editor
- **Then:** Tick prompts with a filename generated from the current timestamp instead of a note title

---

## User Story 002

- **Summary:** Drop a named note straight into the Inbox
- **Status:** Completed

### Use Case

- **As a** Tick user who already knows what to call a note
- **I want to** run `tk new <filename>` and skip the editor prompt
- **so that** I can create the file directly without an extra confirmation step

### Acceptance Criteria

- **Scenario:** Create a named file in the Inbox
- **Given:** I am inside an initialized PARA system
- **When:** I run `tk new my-file`
- **Then:** a file named `my-file.md` is created in `0-Inbox` and Tick prints the path it created

---

## User Story 003

- **Summary:** Scaffold a new project as soon as it starts

### Use Case

- **As a** Tick user starting a new short-term effort
- **I want to** run `tk new --project <filename>`
- **so that** I get a directory ready to hold drafts and attachments, not just a single file

### Acceptance Criteria

- **Scenario:** Create a new project directory
- **Given:** I am inside an initialized PARA system
- **When:** I run `tk new --project website-redesign`
- **Then:** a directory `1-Projects/website-redesign` is created containing an `index.md`, and Tick prints the path to that `index.md`

---

## User Story 004

- **Summary:** Scaffold a new area to track an ongoing responsibility

### Use Case

- **As a** Tick user taking on an ongoing responsibility
- **I want to** run `tk new --area <filename>`
- **so that** I get a directory to hold everything related to maintaining that responsibility over time

### Acceptance Criteria

- **Scenario:** Create a new area directory
- **Given:** I am inside an initialized PARA system
- **When:** I run `tk new --area health`
- **Then:** a directory `2-Areas/health` is created containing an `index.md`, and Tick prints the path to that `index.md`

---

## User Story 005

- **Summary:** File a reference note without the overhead of a directory

### Use Case

- **As a** Tick user saving a topic of ongoing interest
- **I want to** run `tk new --resource <filename>`
- **so that** the note is filed as a single flat file, since it won't accumulate supporting material like a project or area would

### Acceptance Criteria

- **Scenario:** Create a new resource file
- **Given:** I am inside an initialized PARA system
- **When:** I run `tk new --resource recipe-ideas`
- **Then:** a file named `recipe-ideas.md` is created in `3-Resources` and Tick prints the path it created

---

## User Story 006

- **Summary:** Never have to type the file extension

### Use Case

- **As a** Tick user creating notes throughout the day
- **I want to** name files without specifying an extension
- **so that** I don't have to remember or type `.md` every time

### Acceptance Criteria

- **Scenario:** Filename given without an extension
- **Given:** I am inside an initialized PARA system
- **When:** I run `tk new my-file` (or any `tk new` variant) with a filename that has no extension
- **Then:** the created file has `.md` appended automatically

---

## User Story 007

- **Summary:** The editor opens pre-populated with the rendered template, not a blank file

### Use Case

- **As a** Tick user capturing a quick thought in `$EDITOR`
- **I want to** see the category's frontmatter and structure already filled in when the editor opens
- **so that** I get the same consistent structure as every other note in that category, without having to retype boilerplate by hand every time I capture something

### Acceptance Criteria

- **Scenario:** Editor opens pre-populated with the rendered template
- **Given:** I am inside an initialized PARA system with the default `note` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new` with no arguments
- **Then:** `$EDITOR` opens on a scratch file already containing the `note` template rendered with `{{date}}` filled in as today's date and `{{title}}` left empty (the title isn't known yet)
- **and Then:** the editor's cursor is positioned at the `{{cursor}}` mark — the title line — so I can start typing the title immediately

- **Scenario:** My edits are layered onto the pre-populated template, not replacing it
- **Given:** I am inside an initialized PARA system with the default `note` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new`, type a title at the cursor, add body text below the frontmatter, save, and exit the editor
- **Then:** the created file contains the rendered frontmatter, my typed title, and my body text — nothing is stripped or re-rendered after I save

- **Scenario:** Cursor positioning falls back gracefully for editors that don't support it
- **Given:** I am inside an initialized PARA system with the default `note` template
- **and Given:** my `$EDITOR` is set to an editor that doesn't understand the `+<line>` cursor-positioning argument
- **When:** I run `tk new` with no arguments
- **Then:** the editor still opens on the pre-populated scratch file, just without the cursor pre-positioned at the title line

---

## User Story 008

- **Summary:** Named notes render the configured template

### Use Case

- **As a** Tick user creating a note by name instead of through the editor
- **I want to** have the category's template applied to the new file
- **so that** I get the same frontmatter and structure I'd get from any other note in that category, without retyping it

### Acceptance Criteria

- **Scenario:** Named Inbox note renders the `note` template
- **Given:** I am inside an initialized PARA system with the default `note` template
- **When:** I run `tk new my-file`
- **Then:** `0-Inbox/my-file.md` is created with the `note` template rendered into it, `{{title}}` filled in with `my-file` and `{{date}}` filled in with today's date

- **Scenario:** Named resource note renders the `resource` template
- **Given:** I am inside an initialized PARA system with the default `resource` template
- **When:** I run `tk new --resource recipe-ideas`
- **Then:** `3-Resources/recipe-ideas.md` is created with the `resource` template rendered into it, `{{title}}` filled in with `recipe-ideas` and `{{date}}` filled in with today's date

---

## User Story 009

- **Summary:** Scaffolded project and area directories render the configured template into `index.md`

### Use Case

- **As a** Tick user scaffolding a new project or area
- **I want to** have the `project`/`area` template rendered into the generated `index.md`
- **so that** every project and area starts with the same structure (status, standard, etc.) without me typing it by hand

### Acceptance Criteria

- **Scenario:** Scaffolded project index.md renders the `project` template
- **Given:** I am inside an initialized PARA system with the default `project` template
- **When:** I run `tk new --project website-redesign`
- **Then:** `1-Projects/website-redesign/index.md` is created with the `project` template rendered into it, `{{title}}` filled in with `website-redesign` and `{{date}}` filled in with today's date

- **Scenario:** Scaffolded area index.md renders the `area` template
- **Given:** I am inside an initialized PARA system with the default `area` template
- **When:** I run `tk new --area health`
- **Then:** `2-Areas/health/index.md` is created with the `area` template rendered into it, `{{title}}` filled in with `health` and `{{date}}` filled in with today's date

---

## User Story 010

- **Summary:** Capture a thought straight into a project, area, or resource

### Use Case

- **As a** Tick user starting a new project, area, or resource from a fleeting idea
- **I want to** run `tk new --project` (or `--area`/`--resource`) with no filename
- **so that** I can write the content first in `$EDITOR` and let Tick infer the name, without giving up the scaffolding those categories get

### Acceptance Criteria

- **Scenario:** Capture directly into a new project
- **Given:** I am inside an initialized PARA system with the default `project` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new --project` with no filename, and my editor opens pre-populated with the rendered `project` template, cursor at the title line
- **and When:** I type `Website Redesign` at the cursor, save, and exit the editor
- **Then:** Tick prompts `Create "website-redesign"?` with the inferred name pre-filled
- **and Then:** if I accept the prompt, a directory `1-Projects/website-redesign` is created containing an `index.md` with the frontmatter and my heading exactly as I left them, and Tick prints the path to that `index.md`

- **Scenario:** Capture directly into a new area
- **Given:** I am inside an initialized PARA system with the default `area` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new --area` with no filename, and my editor opens pre-populated with the rendered `area` template
- **and When:** I fill in the title, save, exit the editor, and accept the inferred name
- **Then:** a directory `2-Areas/<inferred-name>` is created containing an `index.md` with the frontmatter and my content exactly as I left them, and Tick prints the path to that `index.md`

- **Scenario:** Capture directly into a new resource
- **Given:** I am inside an initialized PARA system with the default `resource` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new --resource` with no filename, and my editor opens pre-populated with the rendered `resource` template
- **and When:** I fill in the title, save, exit the editor, and accept the inferred name
- **Then:** a file `3-Resources/<inferred-name>.md` is created with the frontmatter and my content exactly as I left them, and Tick prints the path it created

- **Scenario:** Leaving the template unmodified still falls back to a timestamp
- **Given:** I am inside an initialized PARA system with the default `project` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new --project` with no filename, save the file exactly as it was pre-populated, and exit the editor
- **Then:** Tick prompts with a name generated from the current timestamp instead of a note title, the same fallback used for a plain `tk new` capture (per story 001)

- **Scenario:** Emptying the file still falls back to a timestamp
- **Given:** I am inside an initialized PARA system with the default `project` template
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new --project` with no filename, delete everything the editor was pre-populated with, save an empty file, and exit the editor
- **Then:** Tick prompts with a name generated from the current timestamp instead of a note title

---

## User Story 011

- **Summary:** Templates can stamp the current time, not just the date

### Use Case

- **As a** Tick user with a custom template
- **I want to** include `{{time}}` in a template
- **so that** notes capture the moment they were created down to the minute, not just the day

### Acceptance Criteria

- **Scenario:** `{{time}}` renders in a named, non-interactive note
- **Given:** I am inside an initialized PARA system with a custom `note` template containing `{{time}}`
- **When:** I run `tk new my-file`
- **Then:** `0-Inbox/my-file.md` is created with `{{time}}` rendered as the current time (e.g. `14:32`)

- **Scenario:** `{{time}}` renders when pre-populating `$EDITOR`
- **Given:** I am inside an initialized PARA system with a custom `note` template containing `{{time}}`
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new` with no arguments
- **Then:** the editor opens pre-populated with `{{time}}` already rendered as the current time, the same as `{{date}}`

---

## User Story 012

- **Summary:** Templates can generate a unique id for Zettelkasten-style notes

### Use Case

- **As a** Tick user practicing Zettelkasten-style note-taking
- **I want to** include `{{uuid}}` in a template
- **so that** every note gets a permanent, unique identifier without me generating one by hand

### Acceptance Criteria

- **Scenario:** `{{uuid}}` renders in a named, non-interactive note
- **Given:** I am inside an initialized PARA system with a custom `note` template containing `{{uuid}}`
- **When:** I run `tk new my-file`
- **Then:** `0-Inbox/my-file.md` is created with `{{uuid}}` rendered as a freshly generated unique id

- **Scenario:** `{{uuid}}` renders when pre-populating `$EDITOR`
- **Given:** I am inside an initialized PARA system with a custom `note` template containing `{{uuid}}`
- **and Given:** my `$EDITOR` environment variable is set
- **When:** I run `tk new` with no arguments
- **Then:** the editor opens pre-populated with `{{uuid}}` already rendered as a freshly generated unique id

- **Scenario:** Each note gets its own id
- **Given:** I am inside an initialized PARA system with a custom `note` template containing `{{uuid}}`
- **When:** I run `tk new first-note` followed by `tk new second-note`
- **Then:** the two created files contain different rendered `{{uuid}}` values

---

## User Story 013

- **Summary:** `--daily` scaffolds (or reopens) today's note without leaving `new`

### Use Case

- **As a** Tick user who thinks of `new`'s flags as the one place category behavior lives
- **I want to** run `tk new --daily` alongside `--project`/`--area`/`--resource`
- **so that** today's note is one more `new` variant instead of a separate command to remember

### Acceptance Criteria

- **Scenario:** `tk new --daily` is equivalent to `tk daily`
- **Given:** I am inside an initialized PARA system with the default `daily` template
- **When:** I run `tk new --daily`
- **Then:** Tick behaves exactly as `tk daily` does (see [daily.md](daily.md)) — creating today's note non-interactively and printing the path if it doesn't exist yet, or opening it in `$EDITOR` if it does

- **Scenario:** `--daily` doesn't accept a filename
- **Given:** I am inside an initialized PARA system
- **When:** I run `tk new some-name --daily`
- **Then:** Tick rejects the command with an error, since the daily note's name is always derived from the current date, not a supplied filename — unlike `--project`/`--area`/`--resource`, which require or accept one

- **Scenario:** `--daily` can't be combined with the other category flags
- **Given:** I am inside an initialized PARA system
- **When:** I run `tk new --daily --project`
- **Then:** Tick rejects the command with an error, since `--daily`, `--project`, `--area`, and `--resource` are mutually exclusive

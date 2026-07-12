# User Stories: `ishi config`

## User Story 001 ✅

- **Summary:** See the config Ishi is actually using, defaults and all
- **Depends on:** Story 002 (config layering/resolution this command displays)

### Use Case

- **As a** Ishi user who isn't sure whether a setting comes from `./.ishi.toml`, `~/.ishi.toml`, or a built-in default
- **I want to** run `ishi config` with no arguments
- **so that** I can see the full effective configuration, and where each setting came from, in one place — without cross-referencing two config files and the docs

### Acceptance Criteria

- **Scenario:** No config files are present
- **Given:** I am inside a PARA system with no `./.ishi.toml` and no `~/.ishi.toml`
- **When:** I run `ishi config`
- **Then:** Ishi prints the built-in default config in `.ishi.toml` (TOML) format, covering `folders`, `defaults`, and `templates`
- **and Then:** every key is marked as coming from the built-in default (e.g. an inline `# default` comment)

- **Scenario:** Only the local config overrides a default
- **Given:** `./.ishi.toml` overrides only the `folders.inbox` key, and there is no `~/.ishi.toml`
- **When:** I run `ishi config`
- **Then:** Ishi prints the full config with `folders.inbox` set to my override, marked as coming from the local config (e.g. `# local`), and every other key marked as a built-in default

- **Scenario:** Both a user-level and local config set the same key
- **Given:** `~/.ishi.toml` sets `templates.daily` and `./.ishi.toml` also sets `templates.daily` to a different value
- **When:** I run `ishi config`
- **Then:** Ishi prints `templates.daily` with the local config's value, marked as coming from the local config (e.g. `# local, overrides user`)

- **Scenario:** Only the user-level config overrides a default
- **Given:** `~/.ishi.toml` sets `templates.note`, and there is no `./.ishi.toml`
- **When:** I run `ishi config`
- **Then:** Ishi prints `templates.note` with the user config's value, marked as coming from the user config (e.g. `# user`)

---

## User Story 002 ✅

- **Summary:** Set personal defaults once, then override them per PARA system
- **Depends on:** None

### Use Case

- **As a** Ishi user who manages more than one PARA system, or shares a repo's config with others
- **I want to** put my personal preferences in `~/.ishi.toml` and only the settings specific to a given system in its local `./.ishi.toml`
- **so that** I don't have to repeat my personal preferences (like templates I like) in every project, while still being able to override them for a specific system

### Acceptance Criteria

- **Scenario:** Only a user-level config exists
- **Given:** `~/.ishi.toml` sets `templates.daily` to a custom value and there is no `./.ishi.toml` in my current PARA system
- **When:** Ishi resolves its configuration (e.g. for `ishi daily` or `ishi config`)
- **Then:** the effective config uses my `~/.ishi.toml` value for `templates.daily`, and built-in defaults for every other key

- **Scenario:** Both a user-level and local config exist, with no overlapping keys
- **Given:** `~/.ishi.toml` sets `templates.daily` and `./.ishi.toml` sets `folders.inbox`
- **When:** Ishi resolves its configuration
- **Then:** the effective config includes both my `templates.daily` override and my `folders.inbox` override, layered on top of the built-in defaults

- **Scenario:** Local config overrides a key also set at the user level
- **Given:** `~/.ishi.toml` sets `templates.daily` to one value and `./.ishi.toml` sets `templates.daily` to a different value
- **When:** Ishi resolves its configuration
- **Then:** the effective config uses the value from `./.ishi.toml`, since the local config takes precedence over the user-level one

- **Scenario:** Neither config file exists
- **Given:** there is no `~/.ishi.toml` and no `./.ishi.toml`
- **When:** Ishi resolves its configuration
- **Then:** the effective config is exactly the built-in defaults

---

## User Story 003 ✅

- **Summary:** Get a starting `.ishi.toml` instead of writing one from scratch
- **Depends on:** Story 002 (built-in default values this scaffolds)

### Use Case

- **As a** Ishi user who wants to customize folder names, the default extension, or note templates
- **I want to** run `ishi config init`
- **so that** I get a `.ishi.toml` populated with the current defaults, ready to edit, instead of having to copy them from documentation by hand

### Acceptance Criteria

- **Scenario:** No local config file exists yet
- **Given:** I am inside a PARA system with no `./.ishi.toml` file
- **When:** I run `ishi config init`
- **Then:** Ishi creates a `./.ishi.toml` containing the default `folders`, `defaults`, and `templates` tables
- **and Then:** Ishi prints the path it created

- **Scenario:** Initializing the user-level config instead
- **Given:** I have no `~/.ishi.toml` file
- **When:** I run `ishi config init -g` (or `--global`)
- **Then:** Ishi creates `~/.ishi.toml` containing the default `folders`, `defaults`, and `templates` tables, instead of touching the local `./.ishi.toml`
- **and Then:** Ishi prints the path it created

---

## User Story 004 ✅

- **Summary:** Don't clobber my customized config by re-running init
- **Depends on:** Story 003 (the `init` command this guards)

### Use Case

- **As a** Ishi user who already has a `.ishi.toml` with my own customizations
- **I want to** be stopped if I accidentally run `ishi config init` again
- **so that** I don't lose changes I've already made to my config

### Acceptance Criteria

- **Scenario:** A local `.ishi.toml` already exists
- **Given:** a `./.ishi.toml` file already exists in my PARA system
- **When:** I run `ishi config init`
- **Then:** Ishi prints an error explaining that `./.ishi.toml` already exists
- **and Then:** the existing file is left untouched

- **Scenario:** A user-level `.ishi.toml` already exists
- **Given:** a `~/.ishi.toml` file already exists
- **When:** I run `ishi config init -g`
- **Then:** Ishi prints an error explaining that `~/.ishi.toml` already exists
- **and Then:** the existing file is left untouched

- **Scenario:** A local config exists, but the user-level one doesn't
- **Given:** `./.ishi.toml` exists but `~/.ishi.toml` does not
- **When:** I run `ishi config init -g`
- **Then:** Ishi creates `~/.ishi.toml` without error, since only the target of `-g` is checked for an existing file

---

## User Story 005 ✅

- **Summary:** Jump straight into editing my config, no need to remember the filename
- **Depends on:** Story 003 (creates defaults when no config exists yet, reused here)

### Use Case

- **As a** Ishi user who wants to tweak my templates or folder names
- **I want to** run `ishi config edit`
- **so that** my `.ishi.toml` opens directly in `$EDITOR` without me having to locate the file myself

### Acceptance Criteria

- **Scenario:** Editing an existing local config
- **Given:** a `./.ishi.toml` file already exists in my PARA system
- **When:** I run `ishi config edit`
- **Then:** Ishi opens that file in `$EDITOR`

- **Scenario:** Editing when no local config exists yet
- **Given:** I am inside a PARA system with no `./.ishi.toml` file
- **When:** I run `ishi config edit`
- **Then:** Ishi creates a `./.ishi.toml` populated with the defaults (as in `ishi config init`) and then opens it in `$EDITOR`

- **Scenario:** Editing the user-level config instead
- **Given:** a `~/.ishi.toml` file already exists
- **When:** I run `ishi config edit -g` (or `--global`)
- **Then:** Ishi opens `~/.ishi.toml` in `$EDITOR`, instead of the local `./.ishi.toml`

- **Scenario:** Editing the user-level config when it doesn't exist yet
- **Given:** I have no `~/.ishi.toml` file
- **When:** I run `ishi config edit -g`
- **Then:** Ishi creates `~/.ishi.toml` populated with the defaults (as in `ishi config init -g`) and then opens it in `$EDITOR`

---

## User Story 006 ✅

- **Summary:** Get autocomplete and validation for `.ishi.toml` in my editor
- **Depends on:** Story 003 (`config init` this schema comment is written by), Story 005 (`config edit`'s no-config-yet path)

### Use Case

- **As a** Ishi user editing `.ishi.toml` in an editor with TOML language support (e.g. VS Code with the Even Better TOML extension)
- **I want to** have my editor autocomplete config keys and flag typos or misplaced values as I type
- **so that** I don't have to consult the docs to remember key names like `templates.daily` or catch a mistyped key only when `ishi` fails to parse the file later

### Acceptance Criteria

- **Scenario:** Generated config points to a schema
- **Given:** I run `ishi config init` (or `ishi config edit` when no config exists yet)
- **When:** Ishi writes the new `.ishi.toml`
- **Then:** the file's first line is a `#:schema` comment pointing to a JSON Schema file describing the `folders`, `defaults`, and `templates` tables
- **and Then:** a Taplo-aware editor (e.g. VS Code with Even Better TOML) uses that schema to offer autocomplete and inline validation for the file, with no extra setup from me

- **Scenario:** Schema file is available on disk
- **Given:** I run `ishi config init`
- **When:** Ishi writes `.ishi.toml` and its `#:schema` comment
- **Then:** the JSON Schema file the comment points to also exists at that path, so the reference resolves without a network fetch

- **Scenario:** Existing config without a schema comment
- **Given:** I have a `.ishi.toml` created before this feature existed, with no `#:schema` comment
- **When:** I run `ishi config edit`
- **Then:** Ishi opens the file as-is, without inserting a `#:schema` comment or otherwise modifying the file's contents

---

## User Story 007 ✅

- **Summary:** `--json` prints the effective config with each key's provenance as a structured field
- **Depends on:** Story 001 (the provenance this restates as data), Story 002 (the layering it reflects)

### Use Case

- **As an** agent driving Ishi on a user's behalf
- **I want to** run `ishi config --json` and get the effective config plus, per key, which layer it came from
- **so that** I can answer "is `archive` renamed locally?" (or any other key) programmatically, instead of parsing inline TOML comments meant for a human skimming the file

### Acceptance Criteria

- **Scenario:** No config files are present
- **Given:** I am inside a PARA system with no `./.ishi.toml` and no `~/.ishi.toml`
- **When:** I run `ishi config --json`
- **Then:** Ishi prints a JSON object with the full effective config (`folders`, `defaults`, `templates`) and, for every key, a provenance value of `default`

- **Scenario:** A local override is reported with its source
- **Given:** `./.ishi.toml` overrides only `folders.inbox`, and there is no `~/.ishi.toml`
- **When:** I run `ishi config --json`
- **Then:** the JSON output's `folders.inbox` entry has provenance `local`, and every other key has provenance `default`

- **Scenario:** A key overridden at both levels reports the local source, matching precedence
- **Given:** `~/.ishi.toml` and `./.ishi.toml` both set `templates.daily` to different values
- **When:** I run `ishi config --json`
- **Then:** the JSON output's `templates.daily` entry has the local config's value and provenance `local`
- **and Then:** this matches the same precedence `ishi config` (human-readable) and Story 002's resolution rules already apply — `--json` is a different encoding of the same resolution, not a different rule

- **Scenario:** A user-level-only override reports its source
- **Given:** `~/.ishi.toml` sets `templates.note`, and there is no `./.ishi.toml`
- **When:** I run `ishi config --json`
- **Then:** the JSON output's `templates.note` entry has provenance `user`

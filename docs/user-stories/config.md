# User Stories: `tk config`

## User Story 001

- **Summary:** See the config Tick is actually using, defaults and all
- **Depends on:** Story 002 (config layering/resolution this command displays)

### Use Case

- **As a** Tick user who isn't sure whether a setting comes from `./.tick.toml`, `~/.tick.toml`, or a built-in default
- **I want to** run `tk config` with no arguments
- **so that** I can see the full effective configuration, and where each setting came from, in one place — without cross-referencing two config files and the docs

### Acceptance Criteria

- **Scenario:** No config files are present
- **Given:** I am inside a PARA system with no `./.tick.toml` and no `~/.tick.toml`
- **When:** I run `tk config`
- **Then:** Tick prints the built-in default config in `.tick.toml` (TOML) format, covering `folders`, `defaults`, and `templates`
- **and Then:** every key is marked as coming from the built-in default (e.g. an inline `# default` comment)

- **Scenario:** Only the local config overrides a default
- **Given:** `./.tick.toml` overrides only the `folders.inbox` key, and there is no `~/.tick.toml`
- **When:** I run `tk config`
- **Then:** Tick prints the full config with `folders.inbox` set to my override, marked as coming from the local config (e.g. `# local`), and every other key marked as a built-in default

- **Scenario:** Both a user-level and local config set the same key
- **Given:** `~/.tick.toml` sets `templates.daily` and `./.tick.toml` also sets `templates.daily` to a different value
- **When:** I run `tk config`
- **Then:** Tick prints `templates.daily` with the local config's value, marked as coming from the local config (e.g. `# local, overrides user`)

- **Scenario:** Only the user-level config overrides a default
- **Given:** `~/.tick.toml` sets `templates.note`, and there is no `./.tick.toml`
- **When:** I run `tk config`
- **Then:** Tick prints `templates.note` with the user config's value, marked as coming from the user config (e.g. `# user`)

---

## User Story 002

- **Summary:** Set personal defaults once, then override them per PARA system
- **Depends on:** None

### Use Case

- **As a** Tick user who manages more than one PARA system, or shares a repo's config with others
- **I want to** put my personal preferences in `~/.tick.toml` and only the settings specific to a given system in its local `./.tick.toml`
- **so that** I don't have to repeat my personal preferences (like templates I like) in every project, while still being able to override them for a specific system

### Acceptance Criteria

- **Scenario:** Only a user-level config exists
- **Given:** `~/.tick.toml` sets `templates.daily` to a custom value and there is no `./.tick.toml` in my current PARA system
- **When:** Tick resolves its configuration (e.g. for `tk daily` or `tk config`)
- **Then:** the effective config uses my `~/.tick.toml` value for `templates.daily`, and built-in defaults for every other key

- **Scenario:** Both a user-level and local config exist, with no overlapping keys
- **Given:** `~/.tick.toml` sets `templates.daily` and `./.tick.toml` sets `folders.inbox`
- **When:** Tick resolves its configuration
- **Then:** the effective config includes both my `templates.daily` override and my `folders.inbox` override, layered on top of the built-in defaults

- **Scenario:** Local config overrides a key also set at the user level
- **Given:** `~/.tick.toml` sets `templates.daily` to one value and `./.tick.toml` sets `templates.daily` to a different value
- **When:** Tick resolves its configuration
- **Then:** the effective config uses the value from `./.tick.toml`, since the local config takes precedence over the user-level one

- **Scenario:** Neither config file exists
- **Given:** there is no `~/.tick.toml` and no `./.tick.toml`
- **When:** Tick resolves its configuration
- **Then:** the effective config is exactly the built-in defaults

---

## User Story 003

- **Summary:** Get a starting `.tick.toml` instead of writing one from scratch
- **Depends on:** Story 002 (built-in default values this scaffolds)

### Use Case

- **As a** Tick user who wants to customize folder names, the default extension, or note templates
- **I want to** run `tk config init`
- **so that** I get a `.tick.toml` populated with the current defaults, ready to edit, instead of having to copy them from documentation by hand

### Acceptance Criteria

- **Scenario:** No local config file exists yet
- **Given:** I am inside a PARA system with no `./.tick.toml` file
- **When:** I run `tk config init`
- **Then:** Tick creates a `./.tick.toml` containing the default `folders`, `defaults`, and `templates` tables
- **and Then:** Tick prints the path it created

- **Scenario:** Initializing the user-level config instead
- **Given:** I have no `~/.tick.toml` file
- **When:** I run `tk config init -g` (or `--global`)
- **Then:** Tick creates `~/.tick.toml` containing the default `folders`, `defaults`, and `templates` tables, instead of touching the local `./.tick.toml`
- **and Then:** Tick prints the path it created

---

## User Story 004

- **Summary:** Don't clobber my customized config by re-running init
- **Depends on:** Story 003 (the `init` command this guards)

### Use Case

- **As a** Tick user who already has a `.tick.toml` with my own customizations
- **I want to** be stopped if I accidentally run `tk config init` again
- **so that** I don't lose changes I've already made to my config

### Acceptance Criteria

- **Scenario:** A local `.tick.toml` already exists
- **Given:** a `./.tick.toml` file already exists in my PARA system
- **When:** I run `tk config init`
- **Then:** Tick prints an error explaining that `./.tick.toml` already exists
- **and Then:** the existing file is left untouched

- **Scenario:** A user-level `.tick.toml` already exists
- **Given:** a `~/.tick.toml` file already exists
- **When:** I run `tk config init -g`
- **Then:** Tick prints an error explaining that `~/.tick.toml` already exists
- **and Then:** the existing file is left untouched

- **Scenario:** A local config exists, but the user-level one doesn't
- **Given:** `./.tick.toml` exists but `~/.tick.toml` does not
- **When:** I run `tk config init -g`
- **Then:** Tick creates `~/.tick.toml` without error, since only the target of `-g` is checked for an existing file

---

## User Story 005

- **Summary:** Jump straight into editing my config, no need to remember the filename
- **Depends on:** Story 003 (creates defaults when no config exists yet, reused here)

### Use Case

- **As a** Tick user who wants to tweak my templates or folder names
- **I want to** run `tk config edit`
- **so that** my `.tick.toml` opens directly in `$EDITOR` without me having to locate the file myself

### Acceptance Criteria

- **Scenario:** Editing an existing local config
- **Given:** a `./.tick.toml` file already exists in my PARA system
- **When:** I run `tk config edit`
- **Then:** Tick opens that file in `$EDITOR`

- **Scenario:** Editing when no local config exists yet
- **Given:** I am inside a PARA system with no `./.tick.toml` file
- **When:** I run `tk config edit`
- **Then:** Tick creates a `./.tick.toml` populated with the defaults (as in `tk config init`) and then opens it in `$EDITOR`

- **Scenario:** Editing the user-level config instead
- **Given:** a `~/.tick.toml` file already exists
- **When:** I run `tk config edit -g` (or `--global`)
- **Then:** Tick opens `~/.tick.toml` in `$EDITOR`, instead of the local `./.tick.toml`

- **Scenario:** Editing the user-level config when it doesn't exist yet
- **Given:** I have no `~/.tick.toml` file
- **When:** I run `tk config edit -g`
- **Then:** Tick creates `~/.tick.toml` populated with the defaults (as in `tk config init -g`) and then opens it in `$EDITOR`

---

## User Story 006

- **Summary:** Get autocomplete and validation for `.tick.toml` in my editor
- **Depends on:** Story 003 (`config init` this schema comment is written by), Story 005 (`config edit`'s no-config-yet path)

### Use Case

- **As a** Tick user editing `.tick.toml` in an editor with TOML language support (e.g. VS Code with the Even Better TOML extension)
- **I want to** have my editor autocomplete config keys and flag typos or misplaced values as I type
- **so that** I don't have to consult the docs to remember key names like `templates.daily` or catch a mistyped key only when `tk` fails to parse the file later

### Acceptance Criteria

- **Scenario:** Generated config points to a schema
- **Given:** I run `tk config init` (or `tk config edit` when no config exists yet)
- **When:** Tick writes the new `.tick.toml`
- **Then:** the file's first line is a `#:schema` comment pointing to a JSON Schema file describing the `folders`, `defaults`, and `templates` tables
- **and Then:** a Taplo-aware editor (e.g. VS Code with Even Better TOML) uses that schema to offer autocomplete and inline validation for the file, with no extra setup from me

- **Scenario:** Schema file is available on disk
- **Given:** I run `tk config init`
- **When:** Tick writes `.tick.toml` and its `#:schema` comment
- **Then:** the JSON Schema file the comment points to also exists at that path, so the reference resolves without a network fetch

- **Scenario:** Existing config without a schema comment
- **Given:** I have a `.tick.toml` created before this feature existed, with no `#:schema` comment
- **When:** I run `tk config edit`
- **Then:** Tick opens the file as-is, without inserting a `#:schema` comment or otherwise modifying the file's contents

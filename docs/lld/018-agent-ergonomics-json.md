# LLD: `--json` for `list`/`status`/`config` — Stories list.md 006, status.md 005, config.md 007

Source: [docs/user-stories/list.md](../user-stories/list.md) Story 006,
[docs/user-stories/status.md](../user-stories/status.md) Story 005,
[docs/user-stories/config.md](../user-stories/config.md) Story 007, sequenced
by [docs/user-stories/agent-ergonomics-map.md](../user-stories/agent-ergonomics-map.md)
as Release 1 (Activity 1 + Activity 2's task). Module boundaries follow
[docs/design.md](../design.md). Corresponds to the roadmap's
"Outstanding stories" bullets for `list.md` Story 006, `status.md` Story
005, and `config.md` Story 007.

## Scope

1. `ishi list <category> --json` prints an array of typed rows (name, title,
   updated_days_ago, path), instead of the aligned text table (`list.md`
   006, scenario 1).
2. Resource/inbox rows resolve `path` to the flat file itself (`list.md`
   006, scenario 2).
3. Archive rows carry origin category as a structured `origin` field,
   separate from `name` — not folded into a `"Projects/old-project"` string
   (`list.md` 006, scenario 3).
4. An empty category (with or without a non-matching filter) prints `[]`
   in `--json` mode, never the human-readable message (`list.md` 006,
   scenarios 4-5).
5. `ishi status --json` prints per-category counts plus the project/area
   per-item breakdown as typed data (`status.md` 005, scenarios 1-2).
6. A `--json` project/area entry omits its reviewed field entirely when the
   item was never reviewed, rather than the human-readable `"never"`
   sentinel (`status.md` 005, scenario 3).
7. Inbox/Resources/Archive stay plain counts (numbers) in `--json`, same
   scope as the human-readable output (`status.md` 005, scenario 4).
8. `ishi config --json` prints the effective config plus, per key, which
   layer it came from (`config.md` 007, scenarios 1-4).
9. A key overridden at both user and local level reports provenance
   `local` in JSON (not the human-readable `"local, overrides user"`
   annotation) — same precedence, a coarser-grained provenance value
   (`config.md` 007, scenario 3).

This LLD also unqualifies `ListedItem::name` for archive rows (previously
`"Projects/old-project"`) in favor of a bare `name` plus a new `origin`
field, since scenario 3 requires `origin` to be structured data separate
from `name`. This is a `list_at`-internal contract change, not new
user-facing scope: `ishi list archive`'s human-readable table is
unaffected — `run_list` reconstructs the qualified `Origin/name` display
string from `name` + `origin` at render time, so `list.md` 002's already-
shipped acceptance criteria keep passing unchanged.

### Out of scope

- `review.md` Story 004 (`ishi review <item> --keep|--archive|--skip`) —
  Release 2 on the story map; depends on this release's `status --json`
  existing but is otherwise unrelated code (`review`/`cli::run_review`).
- `exit-codes.md` Story 001 (semantic exit codes) — Release 3; independent
  of `--json` output shape.
- Any change to the human-readable table/summary/config output formats
  beyond the `ListedItem::name`/`origin` split above — this LLD adds a
  parallel `--json` rendering path, it doesn't redesign the text one.

## `design.md` changes

Not yet applied to `docs/design.md`; deferred until this LLD lands. Once
implemented, `design.md` needs:

- `category` — new `Category::key()` (singular lowercase name: `inbox`,
  `project`, `area`, `resource`, `archive`) used for `list --json`'s
  `origin` field.
- `items` — `ListedItem` gains `path: PathBuf` (already-resolved file
  path, same file `review`/`move` operate on) and `origin: Option<Category>`
  (archive rows only); `name` is unqualified for all categories now,
  including archive.
- `config` — new `Source::json_value()` (collapses `LocalOverridesUser`
  into `"local"`, unlike `comment()`) and `render_effective_json`,
  alongside the existing `render_effective`.
- `cli` — new `run_list_json`, `run_status_json`; `run_list`'s text
  renderer reconstructs the qualified archive display name from
  `name`/`origin` instead of reading a pre-qualified `name`.

## Module designs

### `category` (extends existing module)

```rust
impl Category {
    /// Singular lowercase name for machine-readable output (`list
    /// --json`'s `origin` field). Distinct from `archive_origin_name`
    /// (plural, capitalized, filesystem-facing) and `display_name`
    /// (plural, capitalized, human-facing) — this one is for JSON
    /// consumers to branch on without parsing a display string.
    pub fn key(&self) -> &'static str {
        match self {
            Category::Inbox => "inbox",
            Category::Project => "project",
            Category::Area => "area",
            Category::Resource => "resource",
            Category::Archive => "archive",
        }
    }
}
```

No dependents change — this is additive.

### `items` (extends existing module)

```rust
pub struct ListedItem {
    pub name: String,
    pub title: String,
    pub updated_days_ago: u64,
    pub path: PathBuf,
    /// `Some(origin)` for `Category::Archive` rows, giving the category the
    /// item was archived from; `None` otherwise. Replaces the old
    /// `"Origin/name"`-qualified `name` convention — see Scope.
    pub origin: Option<Category>,
}
```

`build_listed_item` already reads `source_path` (the exact file `list_at`
resolved via `scan_dir`) to infer title/mtime — it now also carries that
path through into `ListedItem::path`, no new I/O.

`list_at`'s `Archive` branch stops building a qualified `name` and instead
passes the bare `name` plus `origin: Some(origin)` into `build_listed_item`.
Since `list.md` 002 requires archive rows sorted by the *qualified* name
(`Projects/old-project` before `Resources/api-notes-v1`, i.e. origin-then-
name order), sorting keys off `(origin.map(Category::archive_origin_name),
name)` instead of `name` alone — same resulting order, computed without
storing the qualified string.

**Existing call-site fallout:** `list_at_archive_qualifies_name_with_origin_category`
and `list_at_archive_missing_origin_subfolder_is_skipped` in `items.rs`
currently assert `items[0].name == "Projects/old-project"`; both need
updating to assert `name == "old-project"` and `origin == Some(Category::Project)`.
No other `items` test touches archive `name` qualification.

### `config` (extends existing module)

```rust
impl Source {
    /// Coarser than `comment()`: collapses `LocalOverridesUser` into
    /// `"local"` for `config --json`'s provenance field, since an agent
    /// branching on provenance only needs "which layer won," not the
    /// human-readable aside about what it overrode (config.md 007,
    /// scenario 3).
    pub fn json_value(self) -> &'static str {
        match self {
            Source::Default => "default",
            Source::User => "user",
            Source::Local | Source::LocalOverridesUser => "local",
        }
    }
}

#[derive(serde::Serialize)]
struct FieldJson<'a, T: serde::Serialize> {
    value: T,
    source: &'a str,
}

#[derive(serde::Serialize)]
struct ConfigJson<'a> {
    folders: FoldersJson<'a>,
    defaults: DefaultsJson<'a>,
    templates: TemplatesJson<'a>,
}

/// Field-for-field mirror of `render_effective`'s output, JSON-encoded.
/// Lives beside `render_effective` (not in `cli`) for the same reason bare
/// `ishi config` already bypasses `cli` — `Config`/`ConfigOrigins` are
/// already fully resolved and infallible to render, no `cli` wrapper
/// needed (see design.md's `cli` section, "Bare `ishi config`...").
pub fn render_effective_json(config: &Config, origins: &ConfigOrigins) -> String
```

`FoldersJson`/`DefaultsJson`/`TemplatesJson` are private structs, one
`FieldJson<String>` per config key, named identically to the TOML keys
`render_effective` already emits (`folders.inbox`, `defaults.extension`,
`templates.note`, ...) so the two renderings stay obviously in sync.
`render_effective_json` builds a `ConfigJson`, serializes with
`serde_json::to_string_pretty`, and `.expect`s (mirrors `render_effective`'s
infallibility — no `Result` needed, the value is always representable).

### `cli` (extends existing module)

```rust
/// `items::list`'s rows as a JSON array — `list.md` 006. Prints `[]` for
/// an empty result (no items, or a filter matching nothing), never the
/// human-readable message.
pub fn run_list_json(
    ws: &Workspace,
    category: Category,
    filter: Option<&str>,
) -> anyhow::Result<String>

/// `items::status`'s report as a JSON object — `status.md` 005. Counts use
/// the same lowercase keys as `Category::key()`; `projects`/`areas` are
/// arrays, `inbox`/`resources`/`archive` stay plain numbers.
pub fn run_status_json(ws: &Workspace) -> anyhow::Result<String>
```

Both are new entry points alongside the existing `run_list`/`run_status`,
not replacements — `main` picks one or the other based on `--json`. Each
defines its own private `#[derive(serde::Serialize)]` row/report structs in
`cli.rs` (`ListRowJson`, `StatusItemJson`, `StatusReportJson`) that borrow
from `items`' plain data rather than deriving `Serialize` on `items`'
structs directly — keeps `items` free of a JSON-shape opinion (it returns
facts; `cli` decides how to encode them for a human or a machine, matching
the existing "cli renders, items doesn't" boundary).

`ListRowJson.origin` and `StatusItemJson.reviewed_days_ago` both use
`#[serde(skip_serializing_if = "Option::is_none")]` — chosen over emitting
`null` so an agent can check key-presence, and because `status.md` 005
scenario 3 explicitly accepts either ("null (or the key is absent)"); the
same convention is applied to `list.md` 006's `origin` field for
consistency between the two commands' JSON rows, even though `list.md`
doesn't spell out the null-vs-absent choice explicitly.

`run_list`'s existing text renderer changes internally: it no longer reads
`item.name` directly for the qualified archive display string — it builds
`format!("{}/{}", origin.archive_origin_name(), item.name)` when
`item.origin.is_some()`, else uses `item.name` bare, for both the column-
width calculation and each printed row. Its return type/behavior/tests
(`list.md` 001-005) are otherwise unchanged.

## Test plan (TDD — write these first)

| Scenario | Test | Module |
|---|---|---|
| Archive `list_at` row has bare name + `origin` | Update `list_at_archive_qualifies_name_with_origin_category` to assert `name == "old-project"`, `origin == Some(Category::Project)` (and same for the Resources row) | `items` (unit, regression for `list.md` 002 contract) |
| Archive `list_at` skip-missing-subfolder still bare-names | Update `list_at_archive_missing_origin_subfolder_is_skipped` the same way | `items` (unit) |
| `ListedItem.path` resolves to `index.md` for directory-style categories | `list_at` on a `Project`, assert `items[0].path` ends in `website-redesign/index.md` | `items` (unit, `list.md` 006 scenario 1) |
| `ListedItem.path` resolves to the flat file for `Resource`/`Inbox` | `list_at` on a `Resource`, assert `items[0].path` is `api-notes.md` directly | `items` (unit, `list.md` 006 scenario 2) |
| `run_list_json` renders name/title/updated_days_ago/path | Call with a project, parse the JSON, assert all four fields | `cli` (unit, `list.md` 006 scenario 1) |
| `run_list_json` archive row has separate `origin`, no qualified name | Call with `Category::Archive`, assert `name == "old-project"` and `origin == "project"` in the parsed JSON | `cli` (unit, `list.md` 006 scenario 3) |
| `run_list_json` non-archive row has no `origin` key | Call with a project, assert the parsed JSON object has no `origin` key | `cli` (unit, consistency with skip-if-none choice above) |
| `run_list_json` empty category prints `[]` | Call on an empty `Resource` category, assert output is exactly `[]` | `cli` (unit, `list.md` 006 scenario 4) |
| `run_list_json` filter matching nothing prints `[]` | Call with a non-matching filter, assert output is `[]` | `cli` (unit, `list.md` 006 scenario 5) |
| `run_list`'s text table still qualifies archive names | Existing `run_list_renders_archive_category_with_qualified_names` still passes unmodified | `cli` (regression, `list.md` 002) |
| `run_status_json` emits all five counts under lowercase keys | Call on a populated workspace, assert `inbox`/`projects`/`areas`/`resources`/`archive` keys with correct counts | `cli` (unit, `status.md` 005 scenario 1) |
| `run_status_json` project/area entries include name/title/updated/reviewed | Call with a project with `last_reviewed` set, assert the entry's four fields | `cli` (unit, `status.md` 005 scenario 2) |
| `run_status_json` never-reviewed entry omits reviewed field | Call with a project with no `last_reviewed`, assert the parsed JSON object has no `reviewed_days_ago` key | `cli` (unit, `status.md` 005 scenario 3) |
| `run_status_json` inbox/resources/archive are plain numbers | Assert `report["inbox"]`/`report["resources"]`/`report["archive"]` parse as JSON numbers, not arrays | `cli` (unit, `status.md` 005 scenario 4) |
| `Category::key()` returns singular lowercase names | Assert all five variants | `category` (unit) |
| `Source::json_value()` collapses `LocalOverridesUser` to `"local"` | Assert all four `Source` variants map correctly, especially `LocalOverridesUser -> "local"` | `config` (unit) |
| `render_effective_json` reports `default` when no config files exist | Resolve with no local/user config, assert every field's `source == "default"` | `config` (unit, `config.md` 007 scenario 1) |
| `render_effective_json` reports `local` for a local-only override | Resolve with `./.ishi.toml` overriding `folders.inbox`, assert that field's `source == "local"`, others `"default"` | `config` (unit, `config.md` 007 scenario 2) |
| `render_effective_json` reports `local` (not the verbose comment) for local-overrides-user | Resolve with both layers setting `templates.daily`, assert `source == "local"` and `value` is the local one | `config` (unit, `config.md` 007 scenario 3) |
| `render_effective_json` reports `user` for a user-only override | Resolve with only `~/.ishi.toml` setting `templates.note`, assert `source == "user"` | `config` (unit, `config.md` 007 scenario 4) |
| `ishi list <category> --json` parses via clap | Extend `main.rs`'s CLI-parsing tests with a `--json` case for `List` | `main` (unit, parse-only) |
| `ishi status --json` parses via clap | Same, for `Status` | `main` (unit, parse-only) |
| `ishi config --json` parses via clap | Same, for bare `Config { action: None, json: true }` | `main` (unit, parse-only) |
| `ishi config init --json` is rejected | Assert the dispatch match returns an error when `json: true` is combined with `action: Some(_)` | `main` (unit) |

## Implementation plan

1. Add `Category::key()` to `category.rs` with its unit test; watch it
   fail, then implement.
2. Add `Source::json_value()` to `config.rs` with its unit test.
3. Change `items::ListedItem` (add `path`, `origin`; unqualify archive
   `name`) and `list_at`'s archive branch/sort key. Update the two
   existing archive tests per the test plan; add the two new `path`
   assertions. Run `cargo test -p ishi items::` to confirm only the
   intended archive-name assertions moved.
4. Add `config::render_effective_json` (plus its private `*Json` structs)
   and its four unit tests.
5. Add `cli::run_list_json` and `cli::run_status_json` (plus their private
   `*Json` structs), and update `cli::run_list`'s text renderer to
   reconstruct the qualified archive name from `name`/`origin`. Write the
   new unit tests first, confirm the existing `run_list_renders_archive_category_with_qualified_names`
   test still passes unmodified.
6. Add `--json: bool` to `main.rs`'s `Commands::List` and change
   `Commands::Status` from a unit variant to `Status { json: bool }`,
   updating the existing `assert_eq!(cli.command, Commands::Status)`-style
   tests to `Commands::Status { json: false }`. Add `json: bool` to
   `Commands::Config`. Wire dispatch: `List`/`Status` branch between the
   `_json` and existing `run_*` calls; `Config`'s bare arm branches between
   `render_effective`/`render_effective_json`, and the `Some(_), json: true`
   arm returns an error. Add the parse-only tests from the test plan.
7. Mark `list.md` Story 006, `status.md` Story 005, and `config.md` Story
   007 `✅` in their story files.
8. Update `docs/roadmap.md`: remove the three shipped bullets from
   "Outstanding stories" under the `agent-ergonomics-map.md` entry (leaving
   Release 2/3's `review.md`/`exit-codes.md` bullets in place).
9. Manual smoke test:
   - `ishi list project --json` in a workspace with a couple of projects —
     confirm `path` points at each `index.md`.
   - `ishi list archive --json` with an archived project — confirm `name`
     is bare and `origin` is `"project"`, and `ishi list archive`
     (no `--json`) still prints `Projects/<name>`.
   - `ishi list resource --json` on an empty Resources folder — confirm
     output is exactly `[]`.
   - `ishi status --json` — confirm counts plus project/area arrays, and
     that a never-reviewed project's entry has no `reviewed_days_ago` key.
   - `ishi config --json` with only `./.ishi.toml` overriding
     `folders.inbox` — confirm that field's `source` is `"local"` and
     every other field is `"default"`.
   - `ishi config init --json` — confirm it errors instead of silently
     ignoring `--json`.
10. `cargo clippy && cargo fmt --check && cargo test` clean before calling
    the three stories done.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::Local;
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::category::{Category, Kind};
use crate::config;
use crate::editor::Editor;
use crate::items;
use crate::workspace::{self, Workspace};

#[derive(Debug, Error)]
pub enum UiError {
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub trait Ui {
    fn confirm(&mut self, prompt: &str, default: &str) -> Result<String, UiError>;

    /// `header` is printed on its own line; `options` are rendered as
    /// `[c]est  [c]est...`, joined by two spaces with no space before the
    /// trailing `?` — e.g. `[k]eep  [a]rchive  [s]kip?` (review.md 001
    /// scenarios 2-3's exact prompt shape). Loops on unrecognized input the
    /// same way the previous single-bracket form did.
    fn choose(&mut self, header: &str, options: &[(char, &str)]) -> Result<char, UiError>;

    /// A plain informational line, no prompt/response — currently only
    /// `review`'s "Nothing to review." message (review.md 001 scenario 4).
    fn info(&mut self, message: &str);
}

pub struct TerminalUi;

impl Ui for TerminalUi {
    fn confirm(&mut self, prompt: &str, default: &str) -> Result<String, UiError> {
        eprint!("{prompt} [{default}] ");
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            Ok(default.to_string())
        } else {
            Ok(trimmed.to_string())
        }
    }

    fn choose(&mut self, header: &str, options: &[(char, &str)]) -> Result<char, UiError> {
        eprintln!("{header}");
        let rendered = options
            .iter()
            .map(|(c, rest)| format!("[{c}]{rest}"))
            .collect::<Vec<_>>()
            .join("  ");
        loop {
            eprint!("  {rendered}? ");
            io::stderr().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if let Some(choice) = input.trim().to_lowercase().chars().next()
                && options.iter().any(|(c, _)| *c == choice)
            {
                return Ok(choice);
            }
        }
    }

    fn info(&mut self, message: &str) {
        println!("{message}");
    }
}

/// Gathers the fields common to every template render — today's date, the
/// current time, and a fresh UUID — computed once so `run_new`'s two arms
/// and `run_daily` don't each redo the same three calls.
fn template_fields() -> (String, String, String) {
    let now = Local::now();
    let today = now.date_naive().format("%Y-%m-%d").to_string();
    let time = now.format("%H:%M").to_string();
    let uuid = Uuid::new_v4().to_string();
    (today, time, uuid)
}

pub fn run_new(
    ws: &Workspace,
    editor: &dyn Editor,
    ui: &mut dyn Ui,
    kind: Kind,
    filename: Option<String>,
    assume_yes: bool,
) -> anyhow::Result<PathBuf> {
    let category = kind.category();
    let template = ws.config.templates.for_kind(kind);
    let path = match filename {
        Some(name) => {
            let (today, time, uuid) = template_fields();
            let rendered =
                config::render(template, &name, &today, &time, &uuid).replace("{{cursor}}", "");
            items::create(ws, category, &name, &rendered)?
        }
        None => {
            let (today, time, uuid) = template_fields();
            let seed = config::render(template, "", &today, &time, &uuid);
            let (content, suggested) = editor.capture(&seed)?;
            let default = if category.is_directory_style() {
                suggested
            } else {
                format!("{suggested}.{}", ws.config.default_extension)
            };
            let chosen = if assume_yes {
                default
            } else {
                ui.confirm(&format!("Create \"{default}\"?"), &default)?
            };
            items::create(ws, category, &chosen, &content)?
        }
    };
    Ok(path)
}

#[derive(Debug)]
pub enum DailyOutcome {
    Created(PathBuf),
    Reopened(PathBuf),
}

/// True if today's daily note already exists — lets `main` print
/// `Opening $EDITOR...` *before* handing control to a blocking editor
/// process, the same convention the no-filename `run_new` path uses.
pub fn daily_note_exists(ws: &Workspace) -> bool {
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    items::item_path(ws, Category::Inbox, &today).exists()
}

pub fn run_daily(ws: &Workspace, editor: &dyn Editor) -> anyhow::Result<DailyOutcome> {
    let (today, time, uuid) = template_fields();
    let path = items::item_path(ws, Category::Inbox, &today);

    if path.exists() {
        editor.open(&path)?;
        Ok(DailyOutcome::Reopened(path))
    } else {
        let rendered = config::render(
            ws.config.templates.for_kind(Kind::Daily),
            &today,
            &today,
            &time,
            &uuid,
        )
        .replace("{{cursor}}", "");
        let created = items::create(ws, Category::Inbox, &today, &rendered)?;
        Ok(DailyOutcome::Created(created))
    }
}

pub fn run_init(
    cwd: &Path,
    name: Option<&str>,
    home_config: Option<&Path>,
) -> anyhow::Result<String> {
    let (target, display) = match name {
        Some(n) => (cwd.join(n), format!("./{n}")),
        None => (cwd.to_path_buf(), ".".to_string()),
    };

    let report = workspace::init(&target)?;

    let mut lines = vec![match report.created.len() {
        5 => format!("Created PARA system in {display}"),
        0 => format!("PARA system in {display} is already complete; no changes made"),
        _ => format!("Created {} in {display}", report.created.join(", ")),
    }];

    let (config, _origins) = config::Config::resolve(&target.join(".ishi.toml"), home_config)?;
    let archive_dir = &config.category_dirs[Category::Archive as usize];

    let editor_report = workspace::write_editor_excludes(&target, archive_dir)?;
    if !editor_report.zed_created {
        lines.push(format!(
            "Manually add \"{archive_dir}\" to .zed/settings.json's file_scan_exclude."
        ));
    }
    if !editor_report.vscode_created {
        lines.push(format!(
            "Manually add \"{archive_dir}\" to .vscode/settings.json's files.exclude/search.exclude."
        ));
    }

    let claude_md_report = workspace::write_claude_md(&target, archive_dir)?;
    if !claude_md_report.created {
        lines.push(format!(
            "Manually add an instruction to CLAUDE.md not to read files under \"{archive_dir}\" unless asked."
        ));
    }

    Ok(lines.join("\n"))
}

/// Writes the default config to `path` and returns the exact confirmation
/// message `main` prints. `display` is the caller-computed human-readable
/// form (`"./.ishi.toml"` or `"~/.ishi.toml"`).
pub fn run_config_init(path: &Path, display: &str) -> anyhow::Result<String> {
    config::init(path)?;
    Ok(format!("Created {display}"))
}

/// Opens `path` in `$EDITOR`, creating it with the default config first
/// (via `config::init`) if it doesn't exist yet. Returns whether it had to
/// create the file first. `main` checks `path.exists()` itself and prints
/// `Created {display}` / `Opening $EDITOR...` *before* calling this
/// function, since it blocks on the editor — mirroring `run_daily`'s
/// "print before handing control to a blocking editor" convention.
pub fn run_config_edit(path: &Path, editor: &dyn Editor) -> anyhow::Result<bool> {
    let created = match config::init(path) {
        Ok(()) => true,
        Err(config::ConfigError::AlreadyExists { .. }) => false,
        Err(e) => return Err(e.into()),
    };
    editor.open(path)?;
    Ok(created)
}

/// Formats a raw day-count the way `list`/`status`/`review` all render
/// ages: `"today"`, `"1 day ago"`, `"N days ago"`.
pub(crate) fn format_age(days: u64) -> String {
    match days {
        0 => "today".to_string(),
        1 => "1 day ago".to_string(),
        n => format!("{n} days ago"),
    }
}

/// Renders `items::list`'s rows as the `NAME`/`TITLE`/`UPDATED` table:
/// header first, then one row per item, each column left-justified to
/// `3 +` the longest value in that column (including its header) so
/// columns line up regardless of content width. When there are no items,
/// returns a message instead of a header-only table: a no-match message if
/// `filter` was given, or a plain empty-category message otherwise.
pub fn run_list(
    ws: &Workspace,
    category: Category,
    filter: Option<&str>,
) -> anyhow::Result<String> {
    let items = items::list(ws, category, filter)?;

    if items.is_empty() {
        return Ok(match filter {
            Some(f) => format!("No items in {} matching \"{f}\".", category.display_name()),
            None => format!("No items in {}.", category.display_name()),
        });
    }

    let display_names: Vec<String> = items
        .iter()
        .map(|i| match i.origin {
            Some(origin) => format!("{}/{}", origin.archive_origin_name(), i.name),
            None => i.name.clone(),
        })
        .collect();
    let ages: Vec<String> = items
        .iter()
        .map(|i| format_age(i.updated_days_ago))
        .collect();

    let name_width = ["NAME"]
        .into_iter()
        .chain(display_names.iter().map(String::as_str))
        .map(str::len)
        .max()
        .unwrap_or(0)
        + 3;
    let title_width = ["TITLE"]
        .into_iter()
        .chain(items.iter().map(|i| i.title.as_str()))
        .map(str::len)
        .max()
        .unwrap_or(0)
        + 3;

    let mut lines = Vec::with_capacity(items.len() + 1);
    lines.push(format!(
        "{:<name_width$}{:<title_width$}UPDATED",
        "NAME", "TITLE"
    ));
    for ((item, name), age) in items.iter().zip(display_names.iter()).zip(ages.iter()) {
        lines.push(format!(
            "{:<name_width$}{:<title_width$}{age}",
            name, item.title
        ));
    }
    Ok(lines.join("\n"))
}

#[derive(Serialize)]
struct ListRowJson {
    name: String,
    title: String,
    updated_days_ago: u64,
    path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<&'static str>,
}

/// `items::list`'s rows as a JSON array — `list.md` 006. Prints `[]` for
/// an empty result (no items, or a filter matching nothing), never the
/// human-readable message.
pub fn run_list_json(
    ws: &Workspace,
    category: Category,
    filter: Option<&str>,
) -> anyhow::Result<String> {
    let items = items::list(ws, category, filter)?;
    let rows: Vec<ListRowJson> = items
        .into_iter()
        .map(|item| ListRowJson {
            name: item.name,
            title: item.title,
            updated_days_ago: item.updated_days_ago,
            path: item.path,
            origin: item.origin.map(|c| c.key()),
        })
        .collect();
    Ok(serde_json::to_string_pretty(&rows).expect("ListRowJson is always representable as JSON"))
}

#[derive(Serialize)]
struct StatusItemJson {
    name: String,
    title: String,
    updated_days_ago: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    reviewed_days_ago: Option<u64>,
}

#[derive(Serialize)]
struct StatusReportJson {
    inbox: usize,
    project: Vec<StatusItemJson>,
    area: Vec<StatusItemJson>,
    resource: usize,
    archive: usize,
}

/// `items::status`'s report as a JSON object — `status.md` 005. Counts use
/// the same lowercase keys as `Category::key()`; `project`/`area` are
/// arrays, `inbox`/`resource`/`archive` stay plain numbers.
pub fn run_status_json(ws: &Workspace) -> anyhow::Result<String> {
    let report = items::status(ws)?;
    let to_json = |items: Vec<items::StatusItem>| -> Vec<StatusItemJson> {
        items
            .into_iter()
            .map(|item| StatusItemJson {
                name: item.name,
                title: item.title,
                updated_days_ago: item.updated_days_ago,
                reviewed_days_ago: item.reviewed_days_ago,
            })
            .collect()
    };
    let json = StatusReportJson {
        inbox: report.counts[Category::Inbox as usize],
        project: to_json(report.projects),
        area: to_json(report.areas),
        resource: report.counts[Category::Resource as usize],
        archive: report.counts[Category::Archive as usize],
    };
    Ok(serde_json::to_string_pretty(&json)
        .expect("StatusReportJson is always representable as JSON"))
}

/// Renders `items::status`'s counts as one `<Category> <count>` line per
/// category, in `Inbox`/`Projects`/`Areas`/`Resources`/`Archive` order,
/// name-column width sized the same way `run_list`'s NAME column is
/// (longest label + 3). `Project`/`Area` also get per-item rows under
/// their count line; `Inbox`/`Resource`/`Archive` stay counts-only.
pub fn run_status(ws: &Workspace) -> anyhow::Result<String> {
    let categories = [
        Category::Inbox,
        Category::Project,
        Category::Area,
        Category::Resource,
        Category::Archive,
    ];
    let report = items::status(ws)?;

    let name_width = categories
        .iter()
        .map(|c| c.display_name().len())
        .max()
        .unwrap_or(0)
        + 3;

    let mut lines = Vec::new();
    for category in categories {
        lines.push(format!(
            "{:<name_width$}{}",
            category.display_name(),
            report.counts[category as usize]
        ));
        match category {
            Category::Project => lines.extend(render_status_items(&report.projects)),
            Category::Area => lines.extend(render_status_items(&report.areas)),
            _ => {}
        }
    }
    Ok(lines.join("\n"))
}

/// Renders `Project`/`Area`'s per-item rows: `` `- `` prefix, Name/Title
/// columns sized the same way `run_list`'s are, then `updated:`/
/// `reviewed:` ages (`reviewed: never` when the item has no
/// `last_reviewed` value). Empty input renders no rows at all.
fn render_status_items(items: &[items::StatusItem]) -> Vec<String> {
    if items.is_empty() {
        return Vec::new();
    }

    let name_width = items.iter().map(|i| i.name.len()).max().unwrap_or(0) + 3;
    let title_width = items.iter().map(|i| i.title.len()).max().unwrap_or(0) + 3;

    items
        .iter()
        .map(|item| {
            let reviewed = match item.reviewed_days_ago {
                Some(days) => format_age(days),
                None => "never".to_string(),
            };
            format!(
                "`- {:<name_width$}{:<title_width$}updated: {}   reviewed: {}",
                item.name,
                item.title,
                format_age(item.updated_days_ago),
                reviewed
            )
        })
        .collect()
}

/// Renders `path` for user-facing messages: relative to the workspace root,
/// with the (possibly custom-configured) numbered category folder replaced
/// by its lowercase canonical name, e.g. `0-Inbox/notes.md` ->
/// `inbox/notes.md`, and — for items filed under `Archive` — the origin
/// subfolder lowercased too, e.g. `4-Archive/Projects/foo` ->
/// `archive/projects/foo`. Falls back to `path` as-is (absolute or
/// otherwise unmodified) if it isn't inside any of the five category
/// folders.
pub fn display_path(ws: &Workspace, path: &Path) -> String {
    for category in [
        Category::Inbox,
        Category::Project,
        Category::Area,
        Category::Resource,
        Category::Archive,
    ] {
        let Ok(rest) = path.strip_prefix(ws.category_dir(category)) else {
            continue;
        };

        let mut display = PathBuf::from(category.display_name().to_lowercase());
        if category == Category::Archive {
            let mut components = rest.components();
            if let Some(origin) = components.next() {
                let word = origin.as_os_str().to_string_lossy();
                let lowered = Category::archivable()
                    .into_iter()
                    .find(|c| c.archive_origin_name() == word)
                    .map(|c| c.archive_origin_name().to_lowercase())
                    .unwrap_or_else(|| word.into_owned());
                display.push(lowered);
            }
            display.push(components.as_path());
        } else {
            display.push(rest);
        }
        return display.display().to_string();
    }
    path.display().to_string()
}

/// Locates `name` via `items::locate`, moves it to `target` via
/// `items::mv`, and returns the exact confirmation message `main` prints:
/// `Moved <source path> to <dest path>` (move.md 001's message shape),
/// with both paths rendered via `display_path` (workspace-root-relative,
/// lowercase category names). When `target == Category::Archive`, prompts for a summary
/// (defaulting per `items::summary_default`) and stamps it via
/// `items::write_summary` before the move — the summary must land in the
/// item's frontmatter before `mv` relocates it (move.md 006). For any other
/// `target`, no prompt, no stamp (move.md 006 scenario 5). When `assume_yes`
/// is set, the summary prompt is skipped and the suggested default is used
/// as-is, for non-interactive use (`--yes`).
pub fn run_move(
    ws: &Workspace,
    ui: &mut dyn Ui,
    name: &str,
    target: Category,
    assume_yes: bool,
) -> anyhow::Result<String> {
    let (source, source_path) = items::locate(ws, name)?
        .ok_or_else(|| anyhow::anyhow!("No item named \"{name}\" found"))?;

    if target == Category::Archive {
        let default = items::summary_default(&source_path, source, name)?;
        let summary = if assume_yes {
            default
        } else {
            ui.confirm(&format!("Summary for {name}?"), &default)?
        };
        items::write_summary(&source_path, source, &summary)?;
    }

    let dest_path = items::mv(ws, source, &source_path, name, target)?;
    Ok(format!(
        "Moved {} to {}",
        display_path(ws, &source_path),
        display_path(ws, &dest_path)
    ))
}

/// Locates `name` via `items::locate` and moves it back to the category it
/// was archived from — sugar for `ishi move <OriginCategory>/<name>
/// <OriginCategory-as-target>`, so un-archiving never requires spelling out
/// a destination the qualified name already encodes (move.md 005's
/// `<OriginCategory>/<name>` addressing). Rejects `name` if it doesn't
/// resolve to an `Archive` item — a bare name matching a live item, or no
/// match at all, is never something to "un-archive".
pub fn run_unarchive(ws: &Workspace, name: &str) -> anyhow::Result<String> {
    let (source, source_path) = items::locate(ws, name)?
        .ok_or_else(|| anyhow::anyhow!("No item named \"{name}\" found"))?;

    if source != Category::Archive {
        anyhow::bail!("\"{name}\" is not archived");
    }

    let origin_name = name
        .split_once('/')
        .map(|(origin, _)| origin)
        .expect("items::locate only resolves Archive for a qualified <OriginCategory>/<name>");
    let target = Category::archivable()
        .into_iter()
        .find(|category| category.archive_origin_name() == origin_name)
        .expect("items::locate only resolves Archive for a recognized origin prefix");

    let dest_path = items::mv(ws, source, &source_path, name, target)?;
    Ok(format!(
        "Moved {} to {}",
        display_path(ws, &source_path),
        display_path(ws, &dest_path)
    ))
}

/// Names of every item in a live (non-`Archive`) category, across all four
/// live categories, sourced from `ws` at call time. Used for `ishi move`/
/// `ishi archive`'s tab-completion — the shapes `items::locate` matches. A
/// basename occurring in more than one live category is qualified as
/// `<category>/<name>` (matching `display_path`'s rendering) so this never
/// offers a bare name `items::locate` would now reject as ambiguous
/// (move.md 006); a basename unique across all four categories stays bare.
/// A category whose directory doesn't exist yet, or that can't be read,
/// contributes no names rather than failing the whole listing.
pub fn live_item_names(ws: &Workspace) -> Vec<String> {
    let named: Vec<(Category, String)> = Category::archivable()
        .into_iter()
        .flat_map(|category| {
            items::list(ws, category, None)
                .map(|items| {
                    items
                        .into_iter()
                        .map(|item| (category, item.name))
                        .collect()
                })
                .unwrap_or_else(|_| Vec::new())
        })
        .collect();

    let mut counts: HashMap<String, usize> = HashMap::new();
    for (_, name) in &named {
        *counts.entry(name.clone()).or_default() += 1;
    }

    named
        .into_iter()
        .map(|(category, name)| {
            if counts[&name] > 1 {
                format!("{}/{name}", category.display_name().to_lowercase())
            } else {
                name
            }
        })
        .collect()
}

/// Qualified `<OriginCategory>/<name>` names of every archived item,
/// sourced from `ws` at call time. Used for `ishi unarchive`'s (and `ishi
/// move`'s) tab-completion — the shape `items::locate` matches via its
/// `name.split_once('/')` branch.
pub fn archived_item_names(ws: &Workspace) -> Vec<String> {
    items::list(ws, Category::Archive, None)
        .map(|items| {
            items
                .into_iter()
                .map(|item| match item.origin {
                    Some(origin) => format!("{}/{}", origin.archive_origin_name(), item.name),
                    None => item.name,
                })
                .collect()
        })
        .unwrap_or_else(|_| Vec::new())
}

/// Whether `candidate` (as produced by `live_item_names`/`archived_item_names`,
/// possibly qualified as `<category>/<name>`) should be offered for the
/// in-progress argument text `current`. True if `current` prefixes the whole
/// candidate (covers bare candidates and fully-typed qualified prefixes like
/// `inbox/meeti`), or if `current` prefixes just the `<name>` part after the
/// last `/` (covers typing a colliding item's bare name, e.g. `meeti`, before
/// any category qualifier).
pub fn completion_candidate_matches(candidate: &str, current: &str) -> bool {
    candidate.starts_with(current)
        || candidate
            .rsplit_once('/')
            .is_some_and(|(_, name)| name.starts_with(current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::editor::EditorError;
    use std::fs;
    use tempfile::tempdir;

    struct FakeEditor {
        content: String,
        suggested: String,
    }

    impl Editor for FakeEditor {
        fn capture(&self, _seed: &str) -> Result<(String, String), EditorError> {
            Ok((self.content.clone(), self.suggested.clone()))
        }

        fn open(&self, _path: &Path) -> Result<(), EditorError> {
            unimplemented!("not exercised by this test")
        }
    }

    struct FakeUi {
        confirm_response: String,
    }

    impl Ui for FakeUi {
        fn confirm(&mut self, _prompt: &str, _default: &str) -> Result<String, UiError> {
            Ok(self.confirm_response.clone())
        }

        fn choose(&mut self, _header: &str, _options: &[(char, &str)]) -> Result<char, UiError> {
            unimplemented!("not exercised by `new` story 001")
        }

        fn info(&mut self, _message: &str) {
            unimplemented!("not exercised by `new` story 001")
        }
    }

    fn workspace(root: &std::path::Path) -> Workspace {
        Workspace {
            root: root.to_path_buf(),
            config: Config::default(),
        }
    }

    fn workspace_with_note_template(root: &std::path::Path, template: &str) -> Workspace {
        let mut config = Config::default();
        config.templates.note = template.to_string();
        Workspace {
            root: root.to_path_buf(),
            config,
        }
    }

    fn contains_hh_mm_time(text: &str) -> bool {
        text.split_whitespace().any(|word| {
            word.len() == 5
                && word.as_bytes()[2] == b':'
                && word[..2].chars().all(|c| c.is_ascii_digit())
                && word[3..].chars().all(|c| c.is_ascii_digit())
        })
    }

    fn contains_uuid(text: &str) -> bool {
        text.split_whitespace()
            .any(|word| uuid::Uuid::parse_str(word).is_ok())
    }

    struct PanicEditor;

    impl Editor for PanicEditor {
        fn capture(&self, _seed: &str) -> Result<(String, String), EditorError> {
            panic!("editor should not be invoked when a filename is given")
        }

        fn open(&self, _path: &Path) -> Result<(), EditorError> {
            unimplemented!("not exercised by this test")
        }
    }

    #[test]
    fn accepts_inferred_filename() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = FakeEditor {
            content: "# Website Improvement Ideas\nbody".to_string(),
            suggested: "website-improvement-ideas".to_string(),
        };
        let mut ui = FakeUi {
            confirm_response: "website-improvement-ideas.md".to_string(),
        };

        let path = run_new(&ws, &editor, &mut ui, Kind::Inbox, None, false).unwrap();

        assert_eq!(
            path,
            dir.path().join("0-Inbox/website-improvement-ideas.md")
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# Website Improvement Ideas\nbody"
        );
    }

    #[test]
    fn overrides_inferred_filename() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = FakeEditor {
            content: "# Website Improvement Ideas\nbody".to_string(),
            suggested: "website-improvement-ideas".to_string(),
        };
        let mut ui = FakeUi {
            confirm_response: "my-custom-name".to_string(),
        };

        let path = run_new(&ws, &editor, &mut ui, Kind::Inbox, None, false).unwrap();

        assert_eq!(path, dir.path().join("0-Inbox/my-custom-name.md"));
    }

    #[test]
    fn empty_note_uses_timestamp_default_path() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = FakeEditor {
            content: String::new(),
            suggested: "20260630-153045".to_string(),
        };
        let mut ui = FakeUi {
            confirm_response: "20260630-153045.md".to_string(),
        };

        let path = run_new(&ws, &editor, &mut ui, Kind::Inbox, None, false).unwrap();

        assert_eq!(path, dir.path().join("0-Inbox/20260630-153045.md"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "");
    }

    #[test]
    fn seeds_editor_with_rendered_note_template() {
        use std::cell::RefCell;

        struct RecordingEditor {
            seen_seed: RefCell<String>,
        }

        impl Editor for RecordingEditor {
            fn capture(&self, seed: &str) -> Result<(String, String), EditorError> {
                *self.seen_seed.borrow_mut() = seed.to_string();
                Ok(("# Title\n".to_string(), "title".to_string()))
            }

            fn open(&self, _path: &Path) -> Result<(), EditorError> {
                unimplemented!("not exercised by this test")
            }
        }

        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = RecordingEditor {
            seen_seed: RefCell::new(String::new()),
        };
        let mut ui = FakeUi {
            confirm_response: "title.md".to_string(),
        };

        run_new(&ws, &editor, &mut ui, Kind::Inbox, None, false).unwrap();

        let seed = editor.seen_seed.borrow();
        assert!(seed.contains("{{cursor}}"));
        assert!(!seed.contains("{{title}}"));
        assert!(!seed.contains("{{date}}"));
    }

    #[test]
    fn captures_into_new_project_directory() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = FakeEditor {
            content: "# Website Redesign\nbody".to_string(),
            suggested: "website-redesign".to_string(),
        };
        let mut ui = FakeUi {
            confirm_response: "website-redesign".to_string(),
        };

        let path = run_new(&ws, &editor, &mut ui, Kind::Project, None, false).unwrap();

        assert_eq!(
            path,
            dir.path().join("1-Projects/website-redesign/index.md")
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# Website Redesign\nbody"
        );
    }

    #[test]
    fn captures_into_new_area_directory() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = FakeEditor {
            content: "# Health\nbody".to_string(),
            suggested: "health".to_string(),
        };
        let mut ui = FakeUi {
            confirm_response: "health".to_string(),
        };

        let path = run_new(&ws, &editor, &mut ui, Kind::Area, None, false).unwrap();

        assert_eq!(path, dir.path().join("2-Areas/health/index.md"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Health\nbody");
    }

    #[test]
    fn captures_into_new_resource_file() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = FakeEditor {
            content: "# Recipe Ideas\nbody".to_string(),
            suggested: "recipe-ideas".to_string(),
        };
        let mut ui = FakeUi {
            confirm_response: "recipe-ideas.md".to_string(),
        };

        let path = run_new(&ws, &editor, &mut ui, Kind::Resource, None, false).unwrap();

        assert_eq!(path, dir.path().join("3-Resources/recipe-ideas.md"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Recipe Ideas\nbody");
    }

    #[test]
    fn project_confirm_prompt_suggests_bare_directory_name_without_extension() {
        use std::cell::RefCell;

        struct RecordingUi {
            seen_default: RefCell<String>,
        }

        impl Ui for RecordingUi {
            fn confirm(&mut self, _prompt: &str, default: &str) -> Result<String, UiError> {
                *self.seen_default.borrow_mut() = default.to_string();
                Ok(default.to_string())
            }

            fn choose(
                &mut self,
                _header: &str,
                _options: &[(char, &str)],
            ) -> Result<char, UiError> {
                unimplemented!("not exercised by this test")
            }

            fn info(&mut self, _message: &str) {
                unimplemented!("not exercised by this test")
            }
        }

        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = FakeEditor {
            content: "# Website Redesign\n".to_string(),
            suggested: "website-redesign".to_string(),
        };
        let mut ui = RecordingUi {
            seen_default: RefCell::new(String::new()),
        };

        run_new(&ws, &editor, &mut ui, Kind::Project, None, false).unwrap();

        assert_eq!(*ui.seen_default.borrow(), "website-redesign");
    }

    #[test]
    fn editor_seed_uses_category_specific_template() {
        use std::cell::RefCell;

        struct RecordingEditor {
            seen_seed: RefCell<String>,
        }

        impl Editor for RecordingEditor {
            fn capture(&self, seed: &str) -> Result<(String, String), EditorError> {
                *self.seen_seed.borrow_mut() = seed.to_string();
                Ok(("# Title\n".to_string(), "title".to_string()))
            }

            fn open(&self, _path: &Path) -> Result<(), EditorError> {
                unimplemented!("not exercised by this test")
            }
        }

        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = RecordingEditor {
            seen_seed: RefCell::new(String::new()),
        };
        let mut ui = FakeUi {
            confirm_response: "title".to_string(),
        };

        run_new(&ws, &editor, &mut ui, Kind::Project, None, false).unwrap();

        let seed = editor.seen_seed.borrow();
        assert!(seed.contains("Status: active"));
    }

    #[test]
    fn named_filename_skips_editor() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = PanicEditor;
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let path = run_new(
            &ws,
            &editor,
            &mut ui,
            Kind::Inbox,
            Some("my-file".to_string()),
            false,
        )
        .unwrap();

        assert_eq!(path, dir.path().join("0-Inbox/my-file.md"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# my-file"));
    }

    #[test]
    fn creates_named_project_directory() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = PanicEditor;
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let path = run_new(
            &ws,
            &editor,
            &mut ui,
            Kind::Project,
            Some("website-redesign".to_string()),
            false,
        )
        .unwrap();

        assert_eq!(
            path,
            dir.path().join("1-Projects/website-redesign/index.md")
        );
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# website-redesign"));
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        assert!(content.contains(&format!("last_updated: {today}")));
    }

    #[test]
    fn creates_named_area_directory() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = PanicEditor;
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let path = run_new(
            &ws,
            &editor,
            &mut ui,
            Kind::Area,
            Some("health".to_string()),
            false,
        )
        .unwrap();

        assert_eq!(path, dir.path().join("2-Areas/health/index.md"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# health"));
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        assert!(content.contains(&format!("last_updated: {today}")));
    }

    #[test]
    fn creates_named_resource_file() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = PanicEditor;
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let path = run_new(
            &ws,
            &editor,
            &mut ui,
            Kind::Resource,
            Some("recipe-ideas".to_string()),
            false,
        )
        .unwrap();

        assert_eq!(path, dir.path().join("3-Resources/recipe-ideas.md"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# recipe-ideas"));
    }

    #[test]
    fn named_note_renders_date_in_frontmatter() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = PanicEditor;
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let path = run_new(
            &ws,
            &editor,
            &mut ui,
            Kind::Inbox,
            Some("my-file".to_string()),
            false,
        )
        .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        assert!(content.contains(&format!("last_updated: {today}")));
    }

    #[test]
    fn named_note_renders_time() {
        let dir = tempdir().unwrap();
        let ws = workspace_with_note_template(dir.path(), "captured at {{time}}\n");
        let editor = PanicEditor;
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let path = run_new(
            &ws,
            &editor,
            &mut ui,
            Kind::Inbox,
            Some("my-file".to_string()),
            false,
        )
        .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(contains_hh_mm_time(&content), "content was: {content}");
    }

    #[test]
    fn editor_capture_renders_time() {
        use std::cell::RefCell;

        struct RecordingEditor {
            seen_seed: RefCell<String>,
        }

        impl Editor for RecordingEditor {
            fn capture(&self, seed: &str) -> Result<(String, String), EditorError> {
                *self.seen_seed.borrow_mut() = seed.to_string();
                Ok(("# Title\n".to_string(), "title".to_string()))
            }

            fn open(&self, _path: &Path) -> Result<(), EditorError> {
                unimplemented!("not exercised by this test")
            }
        }

        let dir = tempdir().unwrap();
        let ws = workspace_with_note_template(dir.path(), "captured at {{time}}\n");
        let editor = RecordingEditor {
            seen_seed: RefCell::new(String::new()),
        };
        let mut ui = FakeUi {
            confirm_response: "title".to_string(),
        };

        run_new(&ws, &editor, &mut ui, Kind::Inbox, None, false).unwrap();

        let seed = editor.seen_seed.borrow();
        assert!(!seed.contains("{{time}}"));
        assert!(contains_hh_mm_time(&seed), "seed was: {seed}");
    }

    #[test]
    fn named_note_renders_uuid() {
        let dir = tempdir().unwrap();
        let ws = workspace_with_note_template(dir.path(), "id: {{uuid}}\n");
        let editor = PanicEditor;
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let path = run_new(
            &ws,
            &editor,
            &mut ui,
            Kind::Inbox,
            Some("my-file".to_string()),
            false,
        )
        .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(contains_uuid(&content), "content was: {content}");
    }

    #[test]
    fn editor_capture_renders_uuid() {
        use std::cell::RefCell;

        struct RecordingEditor {
            seen_seed: RefCell<String>,
        }

        impl Editor for RecordingEditor {
            fn capture(&self, seed: &str) -> Result<(String, String), EditorError> {
                *self.seen_seed.borrow_mut() = seed.to_string();
                Ok(("# Title\n".to_string(), "title".to_string()))
            }

            fn open(&self, _path: &Path) -> Result<(), EditorError> {
                unimplemented!("not exercised by this test")
            }
        }

        let dir = tempdir().unwrap();
        let ws = workspace_with_note_template(dir.path(), "id: {{uuid}}\n");
        let editor = RecordingEditor {
            seen_seed: RefCell::new(String::new()),
        };
        let mut ui = FakeUi {
            confirm_response: "title".to_string(),
        };

        run_new(&ws, &editor, &mut ui, Kind::Inbox, None, false).unwrap();

        let seed = editor.seen_seed.borrow();
        assert!(!seed.contains("{{uuid}}"));
        assert!(contains_uuid(&seed), "seed was: {seed}");
    }

    #[test]
    fn two_notes_get_different_uuids() {
        let dir = tempdir().unwrap();
        let ws = workspace_with_note_template(dir.path(), "id: {{uuid}}\n");
        let editor = PanicEditor;
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let first_path = run_new(
            &ws,
            &editor,
            &mut ui,
            Kind::Inbox,
            Some("first-note".to_string()),
            false,
        )
        .unwrap();
        let second_path = run_new(
            &ws,
            &editor,
            &mut ui,
            Kind::Inbox,
            Some("second-note".to_string()),
            false,
        )
        .unwrap();

        let first_content = fs::read_to_string(&first_path).unwrap();
        let second_content = fs::read_to_string(&second_path).unwrap();
        assert_ne!(first_content, second_content);
    }

    #[test]
    fn run_init_bare_full_create() {
        let dir = tempdir().unwrap();

        let message = run_init(dir.path(), None, None).unwrap();

        assert_eq!(message, "Created PARA system in .");
    }

    #[test]
    fn run_init_named_full_create() {
        let dir = tempdir().unwrap();

        let message = run_init(dir.path(), Some("my-para"), None).unwrap();

        assert_eq!(message, "Created PARA system in ./my-para");
        for name in Config::default().category_dirs {
            assert!(dir.path().join("my-para").join(name).is_dir());
        }
    }

    #[test]
    fn run_init_already_complete() {
        let dir = tempdir().unwrap();
        for name in Config::default().category_dirs {
            fs::create_dir_all(dir.path().join(name)).unwrap();
        }

        let message = run_init(dir.path(), None, None).unwrap();

        assert_eq!(
            message,
            "PARA system in . is already complete; no changes made"
        );
    }

    #[test]
    fn run_init_partial_fill_in() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("0-Inbox")).unwrap();

        let message = run_init(dir.path(), None, None).unwrap();

        assert_eq!(
            message,
            "Created 1-Projects, 2-Areas, 3-Resources, 4-Archive in ."
        );
    }

    #[test]
    fn run_init_bare_tolerates_unrelated_contents() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "hello").unwrap();

        let message = run_init(dir.path(), None, None).unwrap();

        assert_eq!(message, "Created PARA system in .");
        assert_eq!(
            fs::read_to_string(dir.path().join("README.md")).unwrap(),
            "hello"
        );
    }

    #[test]
    fn run_init_named_collision_surfaces_error() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("existing-file"), "").unwrap();

        let err = run_init(dir.path(), Some("existing-file"), None).unwrap_err();

        assert!(err.to_string().contains("existing-file"));
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn run_init_creates_editor_excludes_and_claude_md_on_fresh_target() {
        let dir = tempdir().unwrap();

        let message = run_init(dir.path(), None, None).unwrap();

        assert!(dir.path().join(".zed/settings.json").exists());
        assert!(dir.path().join(".vscode/settings.json").exists());
        assert!(dir.path().join("CLAUDE.md").exists());
        assert_eq!(message, "Created PARA system in .");
    }

    #[test]
    fn run_init_prints_instructions_when_zed_settings_pre_exist() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        fs::write(dir.path().join(".zed/settings.json"), "arbitrary").unwrap();

        let message = run_init(dir.path(), None, None).unwrap();

        assert!(message.contains(".zed/settings.json"));
        assert!(message.contains("4-Archive"));
    }

    #[test]
    fn run_init_prints_instructions_when_vscode_settings_pre_exist() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".vscode")).unwrap();
        fs::write(dir.path().join(".vscode/settings.json"), "arbitrary").unwrap();

        let message = run_init(dir.path(), None, None).unwrap();

        assert!(message.contains(".vscode/settings.json"));
        assert!(message.contains("4-Archive"));
    }

    #[test]
    fn run_init_prints_instructions_when_claude_md_pre_exists() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "arbitrary").unwrap();

        let message = run_init(dir.path(), None, None).unwrap();

        assert!(message.contains("CLAUDE.md"));
        assert!(message.contains("4-Archive"));
    }

    #[test]
    fn run_init_uses_targets_existing_custom_archive_name() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".ishi.toml"),
            "[folders]\narchive = \"9-Attic\"\n",
        )
        .unwrap();

        run_init(dir.path(), None, None).unwrap();

        let zed = fs::read_to_string(dir.path().join(".zed/settings.json")).unwrap();
        assert!(zed.contains("9-Attic"));
        assert!(!zed.contains("4-Archive"));
        let claude_md = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(claude_md.contains("9-Attic"));
    }

    #[test]
    fn run_init_rerun_keeps_printing_instructions() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".zed")).unwrap();
        fs::write(dir.path().join(".zed/settings.json"), "arbitrary").unwrap();

        let first = run_init(dir.path(), None, None).unwrap();
        let second = run_init(dir.path(), None, None).unwrap();

        assert!(first.contains(".zed/settings.json"));
        assert!(second.contains(".zed/settings.json"));
    }

    #[test]
    fn run_config_init_creates_file_and_returns_message() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".ishi.toml");

        let message = run_config_init(&path, "./.ishi.toml").unwrap();

        assert_eq!(message, "Created ./.ishi.toml");
        assert!(path.exists());
    }

    #[test]
    fn run_config_init_surfaces_already_exists_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".ishi.toml");
        fs::write(&path, "custom content").unwrap();

        let err = run_config_init(&path, "./.ishi.toml").unwrap_err();

        assert!(err.to_string().contains(&path.display().to_string()));
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn run_config_edit_opens_existing_file_untouched() {
        use std::cell::RefCell;

        struct RecordingEditor {
            opened_path: RefCell<Option<std::path::PathBuf>>,
        }

        impl Editor for RecordingEditor {
            fn capture(&self, _seed: &str) -> Result<(String, String), EditorError> {
                unimplemented!("not exercised by this test")
            }

            fn open(&self, path: &Path) -> Result<(), EditorError> {
                *self.opened_path.borrow_mut() = Some(path.to_path_buf());
                Ok(())
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join(".ishi.toml");
        fs::write(&path, "custom content").unwrap();
        let editor = RecordingEditor {
            opened_path: RefCell::new(None),
        };

        let created = run_config_edit(&path, &editor).unwrap();

        assert!(!created);
        assert_eq!(fs::read_to_string(&path).unwrap(), "custom content");
        assert_eq!(*editor.opened_path.borrow(), Some(path));
    }

    #[test]
    fn run_config_edit_creates_defaults_then_opens_when_missing() {
        use std::cell::RefCell;

        struct RecordingEditor {
            opened_path: RefCell<Option<std::path::PathBuf>>,
        }

        impl Editor for RecordingEditor {
            fn capture(&self, _seed: &str) -> Result<(String, String), EditorError> {
                unimplemented!("not exercised by this test")
            }

            fn open(&self, path: &Path) -> Result<(), EditorError> {
                *self.opened_path.borrow_mut() = Some(path.to_path_buf());
                Ok(())
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join(".ishi.toml");
        let editor = RecordingEditor {
            opened_path: RefCell::new(None),
        };

        let created = run_config_edit(&path, &editor).unwrap();

        assert!(created);
        let (config, _origins) = Config::resolve(&path, None).unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(*editor.opened_path.borrow(), Some(path));
    }

    #[test]
    fn run_config_edit_surfaces_real_write_error() {
        struct PanicEditorOnOpen;

        impl Editor for PanicEditorOnOpen {
            fn capture(&self, _seed: &str) -> Result<(String, String), EditorError> {
                unimplemented!("not exercised by this test")
            }

            fn open(&self, _path: &Path) -> Result<(), EditorError> {
                panic!("open should not be invoked when config::init fails")
            }
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("missing-parent").join(".ishi.toml");
        let editor = PanicEditorOnOpen;

        let err = run_config_edit(&path, &editor);

        assert!(err.is_err());
    }

    struct PanicOnOpenEditor;

    impl Editor for PanicOnOpenEditor {
        fn capture(&self, _seed: &str) -> Result<(String, String), EditorError> {
            unimplemented!("not exercised by this test")
        }

        fn open(&self, _path: &Path) -> Result<(), EditorError> {
            panic!("open should not be invoked when there's no existing daily note")
        }
    }

    struct RecordingOpenEditor {
        opened_path: std::cell::RefCell<Option<std::path::PathBuf>>,
    }

    impl Editor for RecordingOpenEditor {
        fn capture(&self, _seed: &str) -> Result<(String, String), EditorError> {
            unimplemented!("not exercised by this test")
        }

        fn open(&self, path: &Path) -> Result<(), EditorError> {
            *self.opened_path.borrow_mut() = Some(path.to_path_buf());
            Ok(())
        }
    }

    struct NotSetEditor;

    impl Editor for NotSetEditor {
        fn capture(&self, _seed: &str) -> Result<(String, String), EditorError> {
            unimplemented!("not exercised by this test")
        }

        fn open(&self, _path: &Path) -> Result<(), EditorError> {
            Err(EditorError::NotSet)
        }
    }

    #[test]
    fn run_daily_first_run_creates_non_interactively() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = PanicOnOpenEditor;

        let outcome = run_daily(&ws, &editor).unwrap();

        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        match outcome {
            DailyOutcome::Created(path) => {
                assert_eq!(path, dir.path().join(format!("0-Inbox/{today}.md")));
                let content = fs::read_to_string(&path).unwrap();
                assert!(content.contains(&today));
                assert!(!content.contains("{{cursor}}"));
            }
            DailyOutcome::Reopened(_) => panic!("expected Created on first run"),
        }
    }

    #[test]
    fn run_daily_second_run_reopens_without_re_rendering() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        let existing_path = items::create(&ws, Category::Inbox, &today, "custom content").unwrap();

        let editor = RecordingOpenEditor {
            opened_path: std::cell::RefCell::new(None),
        };
        let outcome = run_daily(&ws, &editor).unwrap();

        match outcome {
            DailyOutcome::Reopened(path) => assert_eq!(path, existing_path),
            DailyOutcome::Created(_) => panic!("expected Reopened on second run"),
        }
        assert_eq!(*editor.opened_path.borrow(), Some(existing_path.clone()));
        assert_eq!(
            fs::read_to_string(&existing_path).unwrap(),
            "custom content"
        );
    }

    #[test]
    fn run_daily_editor_not_set_on_reopen_surfaces_error() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        items::create(&ws, Category::Inbox, &today, "custom content").unwrap();

        let editor = NotSetEditor;
        let err = run_daily(&ws, &editor).unwrap_err();

        assert!(err.to_string().contains("$EDITOR"));
    }

    #[test]
    fn run_daily_filename_is_todays_date() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = PanicOnOpenEditor;

        let outcome = run_daily(&ws, &editor).unwrap();

        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        match outcome {
            DailyOutcome::Created(path) => {
                assert_eq!(path.file_stem().unwrap().to_str().unwrap(), today);
            }
            DailyOutcome::Reopened(_) => panic!("expected Created on first run"),
        }
    }

    #[test]
    fn daily_note_exists_reflects_filesystem() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        assert!(!daily_note_exists(&ws));

        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        items::create(&ws, Category::Inbox, &today, "content").unwrap();

        assert!(daily_note_exists(&ws));
    }

    #[test]
    fn format_age_today() {
        assert_eq!(format_age(0), "today");
    }

    #[test]
    fn format_age_one_day() {
        assert_eq!(format_age(1), "1 day ago");
    }

    #[test]
    fn format_age_many_days() {
        assert_eq!(format_age(21), "21 days ago");
    }

    fn backdate(path: &std::path::Path, days_ago: u64) {
        let modified =
            std::time::SystemTime::now() - std::time::Duration::from_secs(days_ago * 86400);
        let file = fs::File::open(path).unwrap();
        file.set_modified(modified).unwrap();
    }

    #[test]
    fn run_list_renders_directory_style_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path1 = items::create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        backdate(&path1, 2);
        let path2 = items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();
        backdate(&path2, 21);

        let output = run_list(&ws, Category::Project, None).unwrap();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "NAME               TITLE              UPDATED");
        assert_eq!(
            lines[1],
            "my-project         My Project         21 days ago"
        );
        assert_eq!(lines[2], "website-redesign   Website Redesign   2 days ago");
    }

    #[test]
    fn run_list_renders_flat_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path =
            items::create(&ws, Category::Resource, "api-notes", "# API Design Notes\n").unwrap();
        backdate(&path, 5);

        let output = run_list(&ws, Category::Resource, None).unwrap();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "NAME        TITLE              UPDATED");
        assert_eq!(lines[1], "api-notes   API Design Notes   5 days ago");
    }

    #[test]
    fn run_list_renders_single_row_area_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        items::create(&ws, Category::Area, "health", "# Health\n").unwrap();

        let output = run_list(&ws, Category::Area, None).unwrap();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "NAME     TITLE    UPDATED");
        assert_eq!(lines[1], "health   Health   today");
    }

    #[test]
    fn run_list_renders_archive_category_with_qualified_names() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let project_index = dir.path().join("4-Archive/Projects/old-project/index.md");
        fs::create_dir_all(project_index.parent().unwrap()).unwrap();
        fs::write(&project_index, "# Old Project\n").unwrap();
        backdate(&project_index, 120);

        let resource_path = dir.path().join("4-Archive/Resources/api-notes-v1.md");
        fs::create_dir_all(resource_path.parent().unwrap()).unwrap();
        fs::write(&resource_path, "# API Notes v1\n").unwrap();
        backdate(&resource_path, 180);

        let output = run_list(&ws, Category::Archive, None).unwrap();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(
            lines[1],
            "Projects/old-project     Old Project    120 days ago"
        );
        assert_eq!(
            lines[2],
            "Resources/api-notes-v1   API Notes v1   180 days ago"
        );
    }

    #[test]
    fn run_list_json_renders_name_title_updated_and_path() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path = items::create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        backdate(&path, 2);

        let output = run_list_json(&ws, Category::Project, None).unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value[0]["name"], "website-redesign");
        assert_eq!(value[0]["title"], "Website Redesign");
        assert_eq!(value[0]["updated_days_ago"], 2);
        assert_eq!(value[0]["path"], path.display().to_string());
    }

    #[test]
    fn run_list_json_archive_row_has_separate_origin_no_qualified_name() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let project_index = dir.path().join("4-Archive/Projects/old-project/index.md");
        fs::create_dir_all(project_index.parent().unwrap()).unwrap();
        fs::write(&project_index, "# Old Project\n").unwrap();

        let output = run_list_json(&ws, Category::Archive, None).unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value[0]["name"], "old-project");
        assert_eq!(value[0]["origin"], "project");
    }

    #[test]
    fn run_list_json_non_archive_row_has_no_origin_key() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "website-redesign", "").unwrap();

        let output = run_list_json(&ws, Category::Project, None).unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert!(value[0].as_object().unwrap().get("origin").is_none());
    }

    #[test]
    fn run_list_json_empty_category_prints_empty_array() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let output = run_list_json(&ws, Category::Resource, None).unwrap();

        assert_eq!(output, "[]");
    }

    #[test]
    fn run_list_json_filter_matching_nothing_prints_empty_array() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "website-redesign", "").unwrap();

        let output = run_list_json(&ws, Category::Project, Some("nonexistent")).unwrap();

        assert_eq!(output, "[]");
    }

    #[test]
    fn run_list_renders_empty_message_when_category_has_no_items_and_no_filter() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let output = run_list(&ws, Category::Resource, None).unwrap();

        assert_eq!(output, "No items in Resources.");
    }

    #[test]
    fn run_list_renders_no_match_message_when_filter_matches_nothing() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();

        let output = run_list(&ws, Category::Project, Some("nonexistent")).unwrap();

        assert_eq!(output, "No items in Projects matching \"nonexistent\".");
    }

    #[test]
    fn run_list_renders_table_when_filter_matches_something() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path1 = items::create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        backdate(&path1, 2);
        items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();

        let output = run_list(&ws, Category::Project, Some("web")).unwrap();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "NAME               TITLE              UPDATED");
        assert_eq!(lines[1], "website-redesign   Website Redesign   2 days ago");
    }

    #[test]
    fn run_status_renders_five_line_summary() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let archive_dir = dir.path().join("4-Archive");

        items::create(&ws, Category::Inbox, "a", "").unwrap();
        items::create(&ws, Category::Inbox, "b", "").unwrap();

        items::create(&ws, Category::Project, "p1", "").unwrap();
        items::create(&ws, Category::Project, "p2", "").unwrap();
        items::create(&ws, Category::Project, "p3", "").unwrap();

        items::create(&ws, Category::Area, "a1", "").unwrap();
        items::create(&ws, Category::Area, "a2", "").unwrap();

        items::create(&ws, Category::Resource, "r1", "").unwrap();
        items::create(&ws, Category::Resource, "r2", "").unwrap();
        items::create(&ws, Category::Resource, "r3", "").unwrap();
        items::create(&ws, Category::Resource, "r4", "").unwrap();
        items::create(&ws, Category::Resource, "r5", "").unwrap();

        for i in 0..3 {
            let path = archive_dir.join("Inbox").join(format!("i{i}.md"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        for i in 0..3 {
            let path = archive_dir
                .join("Projects")
                .join(format!("p{i}"))
                .join("index.md");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        for i in 0..3 {
            let path = archive_dir
                .join("Areas")
                .join(format!("a{i}"))
                .join("index.md");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        for i in 0..3 {
            let path = archive_dir.join("Resources").join(format!("r{i}.md"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }

        let output = run_status(&ws).unwrap();

        assert_eq!(
            output,
            "Inbox       2\n\
             Projects    3\n\
             `- p1   p1   updated: today   reviewed: never\n\
             `- p2   p2   updated: today   reviewed: never\n\
             `- p3   p3   updated: today   reviewed: never\n\
             Areas       2\n\
             `- a1   a1   updated: today   reviewed: never\n\
             `- a2   a2   updated: today   reviewed: never\n\
             Resources   5\n\
             Archive     12"
        );
    }

    #[test]
    fn run_status_on_empty_system_prints_all_zero_lines() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let output = run_status(&ws).unwrap();

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 5);
        for line in &lines {
            assert!(line.ends_with('0'));
            assert!(!line.contains("- "));
        }
    }

    #[test]
    fn run_status_json_emits_all_five_counts_under_lowercase_keys() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        items::create(&ws, Category::Inbox, "a", "").unwrap();
        items::create(&ws, Category::Project, "p1", "").unwrap();
        items::create(&ws, Category::Area, "a1", "").unwrap();
        items::create(&ws, Category::Resource, "r1", "").unwrap();

        let output = run_status_json(&ws).unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["inbox"], 1);
        assert_eq!(value["project"].as_array().unwrap().len(), 1);
        assert_eq!(value["area"].as_array().unwrap().len(), 1);
        assert_eq!(value["resource"], 1);
        assert_eq!(value["archive"], 0);
    }

    #[test]
    fn run_status_json_project_entry_includes_name_title_updated_and_reviewed() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path = items::create(
            &ws,
            Category::Project,
            "website-redesign",
            "---\nlast_reviewed: 2020-01-01\n---\n# Website Redesign\n",
        )
        .unwrap();
        backdate(&path, 2);

        let output = run_status_json(&ws).unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();

        let entry = &value["project"][0];
        assert_eq!(entry["name"], "website-redesign");
        assert_eq!(entry["title"], "Website Redesign");
        assert_eq!(entry["updated_days_ago"], 2);
        assert!(entry["reviewed_days_ago"].is_number());
    }

    #[test]
    fn run_status_json_never_reviewed_entry_omits_reviewed_field() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();

        let output = run_status_json(&ws).unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();

        let entry = value["project"][0].as_object().unwrap();
        assert!(!entry.contains_key("reviewed_days_ago"));
    }

    #[test]
    fn run_status_json_inbox_resource_archive_are_plain_numbers() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let output = run_status_json(&ws).unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();

        assert!(value["inbox"].is_number());
        assert!(value["resource"].is_number());
        assert!(value["archive"].is_number());
    }

    #[test]
    fn run_status_renders_project_rows_under_the_count_line() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let path1 = items::create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        backdate(&path1, 2);
        items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();

        let output = run_status(&ws).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        let projects_idx = lines
            .iter()
            .position(|l| l.starts_with("Projects"))
            .unwrap();
        assert!(lines[projects_idx + 1].starts_with("`- my-project"));
        assert!(lines[projects_idx + 1].contains("reviewed: never"));
        assert!(lines[projects_idx + 2].starts_with("`- website-redesign"));
        assert!(lines[projects_idx + 2].contains("updated: 2 days ago"));
    }

    #[test]
    fn run_status_renders_area_rows_under_the_count_line() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        items::create(&ws, Category::Area, "health", "# Health\n").unwrap();

        let output = run_status(&ws).unwrap();
        let lines: Vec<&str> = output.lines().collect();

        let areas_idx = lines.iter().position(|l| l.starts_with("Areas")).unwrap();
        assert!(lines[areas_idx + 1].starts_with("`- health"));
    }

    #[test]
    fn run_status_row_falls_back_title_to_name() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        items::create(&ws, Category::Project, "quick-idea", "no heading here").unwrap();

        let output = run_status(&ws).unwrap();

        assert!(output.contains("`- quick-idea   quick-idea"));
    }

    #[test]
    fn run_status_count_only_categories_have_no_rows_following() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        items::create(&ws, Category::Inbox, "a", "").unwrap();
        items::create(&ws, Category::Resource, "r1", "").unwrap();

        let output = run_status(&ws).unwrap();

        assert!(!output.contains("`- "));
    }

    #[test]
    fn run_status_row_shows_reviewed_days_ago() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let today = chrono::Local::now().date_naive();
        let last_reviewed = today - chrono::Duration::days(3);

        let path = items::create(
            &ws,
            Category::Project,
            "website-redesign",
            &format!(
                "---\nlast_reviewed: {}\n---\n# Website Redesign\n",
                last_reviewed.format("%Y-%m-%d")
            ),
        )
        .unwrap();
        backdate(&path, 2);

        let output = run_status(&ws).unwrap();

        assert!(output.contains("updated: 2 days ago   reviewed: 3 days ago"));
    }

    #[test]
    fn run_status_row_shows_reviewed_never() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        items::create(&ws, Category::Project, "my-project", "# My Project\n").unwrap();

        let output = run_status(&ws).unwrap();

        assert!(output.contains("reviewed: never"));
    }

    #[test]
    fn run_move_returns_moved_message() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Inbox, "my-file", "hello").unwrap();
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let message = run_move(&ws, &mut ui, "my-file", Category::Project, false).unwrap();

        assert_eq!(
            message,
            "Moved inbox/my-file.md to projects/my-file/index.md"
        );
    }

    #[test]
    fn run_move_unarchives_a_directory_item() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let archived = dir
            .path()
            .join("4-Archive/Projects/website-redesign/index.md");
        fs::create_dir_all(archived.parent().unwrap()).unwrap();
        fs::write(&archived, "# Website Redesign\n").unwrap();
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let message = run_move(
            &ws,
            &mut ui,
            "Projects/website-redesign",
            Category::Project,
            false,
        )
        .unwrap();

        let dest_path = dir.path().join("1-Projects/website-redesign");
        assert!(message.contains("Moved"));
        assert_eq!(
            fs::read_to_string(dest_path.join("index.md")).unwrap(),
            "# Website Redesign\n"
        );
    }

    #[test]
    fn run_move_rejects_rearchiving_a_directory_style_archived_item() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let archived = dir
            .path()
            .join("4-Archive/Projects/website-redesign/index.md");
        fs::create_dir_all(archived.parent().unwrap()).unwrap();
        fs::write(&archived, "# Website Redesign\n").unwrap();
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let err = run_move(
            &ws,
            &mut ui,
            "Projects/website-redesign",
            Category::Archive,
            true,
        )
        .unwrap_err();

        assert!(err.to_string().contains("already archived"));
        assert!(archived.is_file());
    }

    #[test]
    fn run_move_errors_when_no_item_matches_name() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let err = run_move(&ws, &mut ui, "nonexistent", Category::Project, false).unwrap_err();

        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn run_move_rejects_ambiguous_bare_name_and_moves_nothing() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Inbox, "meeting-notes", "").unwrap();
        items::create(&ws, Category::Resource, "meeting-notes", "").unwrap();
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let err = run_move(&ws, &mut ui, "meeting-notes", Category::Archive, false).unwrap_err();

        assert!(err.to_string().contains("ambiguous"));
        assert!(dir.path().join("0-Inbox/meeting-notes.md").is_file());
        assert!(dir.path().join("3-Resources/meeting-notes.md").is_file());
        assert!(!dir.path().join("4-Archive").exists());
    }

    #[test]
    fn run_move_qualified_name_moves_only_that_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Inbox, "meeting-notes", "").unwrap();
        items::create(&ws, Category::Resource, "meeting-notes", "").unwrap();
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        run_move(
            &ws,
            &mut ui,
            "resources/meeting-notes",
            Category::Archive,
            true,
        )
        .unwrap();

        assert!(dir.path().join("0-Inbox/meeting-notes.md").is_file());
        assert!(!dir.path().join("3-Resources/meeting-notes.md").exists());
        assert!(
            dir.path()
                .join("4-Archive/Resources/meeting-notes.md")
                .is_file()
        );
    }

    #[test]
    fn run_move_rejects_unwrapping_project_into_inbox() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "website-redesign", "# Site\n").unwrap();
        let mut ui = FakeUi {
            confirm_response: String::new(),
        };

        let err = run_move(&ws, &mut ui, "website-redesign", Category::Inbox, false).unwrap_err();

        assert!(err.to_string().contains("not yet supported"));
        assert!(
            dir.path()
                .join("1-Projects/website-redesign/index.md")
                .is_file()
        );
        assert!(!dir.path().join("0-Inbox/website-redesign.md").exists());
    }

    #[test]
    fn run_move_to_archive_prompts_and_stamps_default_summary() {
        use std::cell::RefCell;

        struct RecordingUi {
            seen_prompt: RefCell<String>,
            seen_default: RefCell<String>,
        }

        impl Ui for RecordingUi {
            fn confirm(&mut self, prompt: &str, default: &str) -> Result<String, UiError> {
                *self.seen_prompt.borrow_mut() = prompt.to_string();
                *self.seen_default.borrow_mut() = default.to_string();
                Ok(default.to_string())
            }

            fn choose(
                &mut self,
                _header: &str,
                _options: &[(char, &str)],
            ) -> Result<char, UiError> {
                unimplemented!("not exercised by this test")
            }

            fn info(&mut self, _message: &str) {
                unimplemented!("not exercised by this test")
            }
        }

        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        let mut ui = RecordingUi {
            seen_prompt: RefCell::new(String::new()),
            seen_default: RefCell::new(String::new()),
        };

        run_move(&ws, &mut ui, "website-redesign", Category::Archive, false).unwrap();

        assert_eq!(*ui.seen_prompt.borrow(), "Summary for website-redesign?");
        assert_eq!(*ui.seen_default.borrow(), "Website Redesign");

        let dest = dir
            .path()
            .join("4-Archive/Projects/website-redesign/index.md");
        let content = fs::read_to_string(&dest).unwrap();
        assert!(content.contains("summary: \"Website Redesign\""));
    }

    #[test]
    fn run_unarchive_restores_a_directory_item_to_its_origin_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let archived = dir
            .path()
            .join("4-Archive/Projects/website-redesign/index.md");
        fs::create_dir_all(archived.parent().unwrap()).unwrap();
        fs::write(&archived, "# Website Redesign\n").unwrap();

        let message = run_unarchive(&ws, "Projects/website-redesign").unwrap();

        let dest_path = dir.path().join("1-Projects/website-redesign");
        assert_eq!(
            message,
            "Moved archive/projects/website-redesign to projects/website-redesign"
        );
        assert_eq!(
            fs::read_to_string(dest_path.join("index.md")).unwrap(),
            "# Website Redesign\n"
        );
    }

    #[test]
    fn run_unarchive_restores_a_flat_item_to_its_origin_category() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let archived = dir.path().join("4-Archive/Resources/my-file.md");
        fs::create_dir_all(archived.parent().unwrap()).unwrap();
        fs::write(&archived, "hello").unwrap();

        let message = run_unarchive(&ws, "Resources/my-file").unwrap();

        let dest_path = dir.path().join("3-Resources/my-file.md");
        assert_eq!(
            message,
            "Moved archive/resources/my-file.md to resources/my-file.md"
        );
        assert_eq!(fs::read_to_string(&dest_path).unwrap(), "hello");
    }

    #[test]
    fn run_unarchive_rejects_a_bare_name_matching_a_live_item() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Inbox, "my-file", "hello").unwrap();

        let err = run_unarchive(&ws, "my-file").unwrap_err();

        assert!(err.to_string().contains("not archived"));
        assert!(dir.path().join("0-Inbox/my-file.md").is_file());
    }

    #[test]
    fn run_unarchive_errors_when_no_item_matches_name() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        let err = run_unarchive(&ws, "Projects/nonexistent").unwrap_err();

        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn run_move_to_archive_stamps_custom_summary_not_default() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(
            &ws,
            Category::Project,
            "website-redesign",
            "# Website Redesign\n",
        )
        .unwrap();
        let mut ui = FakeUi {
            confirm_response: "A custom summary".to_string(),
        };

        run_move(&ws, &mut ui, "website-redesign", Category::Archive, false).unwrap();

        let dest = dir
            .path()
            .join("4-Archive/Projects/website-redesign/index.md");
        let content = fs::read_to_string(&dest).unwrap();
        assert!(content.contains("summary: \"A custom summary\""));
        assert!(!content.contains("Website Redesign\"\n"));
    }

    #[test]
    fn run_move_to_non_archive_does_not_prompt_or_stamp() {
        struct PanicUi;

        impl Ui for PanicUi {
            fn confirm(&mut self, _prompt: &str, _default: &str) -> Result<String, UiError> {
                panic!("confirm should not be called for a non-archive move")
            }

            fn choose(
                &mut self,
                _header: &str,
                _options: &[(char, &str)],
            ) -> Result<char, UiError> {
                unimplemented!("not exercised by this test")
            }

            fn info(&mut self, _message: &str) {
                unimplemented!("not exercised by this test")
            }
        }

        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Inbox, "my-file", "hello").unwrap();
        let mut ui = PanicUi;

        run_move(&ws, &mut ui, "my-file", Category::Project, false).unwrap();

        let dest = dir.path().join("1-Projects/my-file/index.md");
        let content = fs::read_to_string(&dest).unwrap();
        assert!(!content.contains("summary:"));
    }

    #[test]
    fn run_status_area_row_shows_reviewed_days_ago() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let today = chrono::Local::now().date_naive();
        let last_reviewed = today - chrono::Duration::days(4);

        items::create(
            &ws,
            Category::Area,
            "finances",
            &format!(
                "---\nlast_reviewed: {}\n---\n# Finances\n",
                last_reviewed.format("%Y-%m-%d")
            ),
        )
        .unwrap();

        let output = run_status(&ws).unwrap();

        assert!(output.contains("reviewed: 4 days ago"));
    }

    #[test]
    fn live_item_names_lists_names_across_all_four_live_categories() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Inbox, "my-file", "hello").unwrap();
        items::create(&ws, Category::Project, "website-redesign", "").unwrap();
        items::create(&ws, Category::Area, "finances", "").unwrap();
        items::create(&ws, Category::Resource, "recipe-ideas", "").unwrap();

        let mut names = live_item_names(&ws);
        names.sort();

        assert_eq!(
            names,
            vec!["finances", "my-file", "recipe-ideas", "website-redesign"]
        );
    }

    #[test]
    fn live_item_names_on_uninitialized_workspace_is_empty() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());

        assert_eq!(live_item_names(&ws), Vec::<String>::new());
    }

    #[test]
    fn live_item_names_qualifies_colliding_basenames() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Inbox, "meeting-notes", "").unwrap();
        items::create(&ws, Category::Resource, "meeting-notes", "").unwrap();

        let mut names = live_item_names(&ws);
        names.sort();

        assert_eq!(
            names,
            vec!["inbox/meeting-notes", "resources/meeting-notes"]
        );
    }

    #[test]
    fn live_item_names_qualifies_only_colliding_names() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Inbox, "meeting-notes", "").unwrap();
        items::create(&ws, Category::Resource, "meeting-notes", "").unwrap();
        items::create(&ws, Category::Project, "website-redesign", "").unwrap();

        let mut names = live_item_names(&ws);
        names.sort();

        assert_eq!(
            names,
            vec![
                "inbox/meeting-notes",
                "resources/meeting-notes",
                "website-redesign"
            ]
        );
    }

    #[test]
    fn archived_item_names_lists_qualified_names_from_multiple_origins() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let project_path = items::create(&ws, Category::Project, "website-redesign", "").unwrap();
        let project_dir = project_path.parent().unwrap().to_path_buf();
        items::mv(
            &ws,
            Category::Project,
            &project_dir,
            "website-redesign",
            Category::Archive,
        )
        .unwrap();
        let resource_path = items::create(&ws, Category::Resource, "my-file", "hello").unwrap();
        items::mv(
            &ws,
            Category::Resource,
            &resource_path,
            "my-file",
            Category::Archive,
        )
        .unwrap();

        let mut names = archived_item_names(&ws);
        names.sort();

        assert_eq!(
            names,
            vec!["Projects/website-redesign", "Resources/my-file"]
        );
    }

    #[test]
    fn archived_item_names_excludes_live_items() {
        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        items::create(&ws, Category::Project, "website-redesign", "").unwrap();
        let resource_path = items::create(&ws, Category::Resource, "my-file", "hello").unwrap();
        items::mv(
            &ws,
            Category::Resource,
            &resource_path,
            "my-file",
            Category::Archive,
        )
        .unwrap();

        let names = archived_item_names(&ws);

        assert_eq!(names, vec!["Resources/my-file"]);
    }

    #[test]
    fn completion_candidate_matches_matches_qualified_candidates_name_segment() {
        assert!(completion_candidate_matches("inbox/meeting-notes", "meeti"));
    }

    #[test]
    fn completion_candidate_matches_matches_full_qualified_prefix() {
        assert!(completion_candidate_matches(
            "inbox/meeting-notes",
            "inbox/meeti"
        ));
        assert!(!completion_candidate_matches(
            "resources/meeting-notes",
            "inbox/meeti"
        ));
    }

    #[test]
    fn completion_candidate_matches_matches_bare_candidate_prefix() {
        assert!(completion_candidate_matches("website-redesign", "website"));
    }

    #[test]
    fn completion_candidate_matches_rejects_non_prefix() {
        assert!(!completion_candidate_matches("inbox/meeting-notes", "zzz"));
    }
}

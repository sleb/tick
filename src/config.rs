use std::fs;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Indexed by `Category as usize` (Inbox, Project, Area, Resource, Archive).
    pub category_dirs: [String; 5],
    pub default_extension: String,
    pub templates: Templates,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            category_dirs: [
                "0-Inbox",
                "1-Projects",
                "2-Areas",
                "3-Resources",
                "4-Archive",
            ]
            .map(String::from),
            default_extension: "md".to_string(),
            templates: Templates::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Templates {
    pub note: String,
    pub daily: String,
    pub project: String,
    pub area: String,
    pub resource: String,
}

impl Default for Templates {
    fn default() -> Self {
        Self {
            note: "---\nlast_updated: {{date}}\n---\n# {{cursor}}{{title}}\n".to_string(),
            daily: "---\ndate: {{date}}\nlast_updated: {{date}}\n---\n# {{date}}\n\n## Tasks\n\n[ ] -\n\n## Notes\n\n{{cursor}}\n".to_string(),
            project:
                "---\nlast_updated: {{date}}\n---\n\n# {{cursor}}{{title}}\n\nStatus: active\n"
                    .to_string(),
            area: "---\nlast_updated: {{date}}\n---\n\n# {{cursor}}{{title}}\n\nStandard:\n"
                .to_string(),
            resource: "---\nlast_updated: {{date}}\n---\n\n# {{cursor}}{{title}}\n".to_string(),
        }
    }
}

impl Templates {
    /// Maps a `Kind` to the template used when creating that kind of item.
    /// Total — there's no `Kind::Archive` to be missing a template for.
    pub fn for_kind(&self, kind: crate::category::Kind) -> &str {
        use crate::category::Kind;
        match kind {
            Kind::Inbox => &self.note,
            Kind::Daily => &self.daily,
            Kind::Project => &self.project,
            Kind::Area => &self.area,
            Kind::Resource => &self.resource,
        }
    }
}

/// Fills in `{{date}}`, `{{title}}`, `{{time}}`, and `{{uuid}}` in
/// `template`. Leaves `{{cursor}}` untouched — interpreting that marker
/// (positioning the editor's cursor, then stripping it) is `Editor`'s job,
/// not the renderer's.
pub fn render(template: &str, title: &str, date: &str, time: &str, uuid: &str) -> String {
    template
        .replace("{{date}}", date)
        .replace("{{title}}", title)
        .replace("{{time}}", time)
        .replace("{{uuid}}", uuid)
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read {path}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse {path}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("{path} already exists")]
    AlreadyExists { path: String },
    #[error("failed to write {path}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Default,
    User,
    Local,
    LocalOverridesUser,
}

impl Source {
    /// The exact annotation `tk config` prints after each key, per
    /// config.md 001 (`# default`, `# user`, `# local`, `# local, overrides user`).
    pub fn comment(self) -> &'static str {
        match self {
            Source::Default => "default",
            Source::User => "user",
            Source::Local => "local",
            Source::LocalOverridesUser => "local, overrides user",
        }
    }
}

pub struct ConfigOrigins {
    /// Indexed by `Category as usize`, same convention as `Config::category_dirs`.
    pub category_dirs: [Source; 5],
    pub default_extension: Source,
    pub templates: TemplateOrigins,
}

pub struct TemplateOrigins {
    pub note: Source,
    pub daily: Source,
    pub project: Source,
    pub area: Source,
    pub resource: Source,
}

#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    folders: Option<RawFolders>,
    defaults: Option<RawDefaults>,
    templates: Option<RawTemplates>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFolders {
    inbox: Option<String>,
    projects: Option<String>,
    areas: Option<String>,
    resources: Option<String>,
    archive: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawDefaults {
    extension: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawTemplates {
    note: Option<String>,
    daily: Option<String>,
    project: Option<String>,
    area: Option<String>,
    resource: Option<String>,
}

fn read_raw(path: &Path) -> Result<RawConfig, ConfigError> {
    if !path.exists() {
        return Ok(RawConfig::default());
    }
    let raw = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.display().to_string(),
        source,
    })?;
    toml::from_str(&raw).map_err(|source| ConfigError::Parse {
        path: path.display().to_string(),
        source,
    })
}

/// `local` wins over `user`, which wins over `default`. The `Source`
/// returned distinguishes "local, but nothing to override" from "local,
/// overriding a value the user config set" — the two different labels
/// config.md 001 requires for the same effective value.
fn merge<T>(default: T, user: Option<T>, local: Option<T>) -> (T, Source) {
    let source = match (&user, &local) {
        (_, Some(_)) if user.is_some() => Source::LocalOverridesUser,
        (_, Some(_)) => Source::Local,
        (Some(_), None) => Source::User,
        (None, None) => Source::Default,
    };
    let value = local.or(user).unwrap_or(default);
    (value, source)
}

impl Config {
    /// Reads `local_path` and, if given, `home_path`, and layers them over
    /// `Config::default()` — local takes precedence over user, user over
    /// the built-in default, independently per key. Neither file needs to
    /// exist; a missing file behaves as if it set no keys at all.
    pub fn resolve(
        local_path: &Path,
        home_path: Option<&Path>,
    ) -> Result<(Config, ConfigOrigins), ConfigError> {
        let local = read_raw(local_path)?;
        let user = match home_path {
            Some(p) => read_raw(p)?,
            None => RawConfig::default(),
        };
        let defaults = Config::default();

        let local_folders = local.folders.unwrap_or_default();
        let user_folders = user.folders.unwrap_or_default();
        let (inbox, inbox_src) = merge(
            defaults.category_dirs[0].clone(),
            user_folders.inbox,
            local_folders.inbox,
        );
        let (projects, projects_src) = merge(
            defaults.category_dirs[1].clone(),
            user_folders.projects,
            local_folders.projects,
        );
        let (areas, areas_src) = merge(
            defaults.category_dirs[2].clone(),
            user_folders.areas,
            local_folders.areas,
        );
        let (resources, resources_src) = merge(
            defaults.category_dirs[3].clone(),
            user_folders.resources,
            local_folders.resources,
        );
        let (archive, archive_src) = merge(
            defaults.category_dirs[4].clone(),
            user_folders.archive,
            local_folders.archive,
        );

        let (extension, extension_src) = merge(
            defaults.default_extension.clone(),
            user.defaults.unwrap_or_default().extension,
            local.defaults.unwrap_or_default().extension,
        );

        let local_templates = local.templates.unwrap_or_default();
        let user_templates = user.templates.unwrap_or_default();
        let (note, note_src) = merge(
            defaults.templates.note.clone(),
            user_templates.note,
            local_templates.note,
        );
        let (daily, daily_src) = merge(
            defaults.templates.daily.clone(),
            user_templates.daily,
            local_templates.daily,
        );
        let (project, project_src) = merge(
            defaults.templates.project.clone(),
            user_templates.project,
            local_templates.project,
        );
        let (area, area_src) = merge(
            defaults.templates.area.clone(),
            user_templates.area,
            local_templates.area,
        );
        let (resource, resource_src) = merge(
            defaults.templates.resource.clone(),
            user_templates.resource,
            local_templates.resource,
        );

        Ok((
            Config {
                category_dirs: [inbox, projects, areas, resources, archive],
                default_extension: extension,
                templates: Templates {
                    note,
                    daily,
                    project,
                    area,
                    resource,
                },
            },
            ConfigOrigins {
                category_dirs: [
                    inbox_src,
                    projects_src,
                    areas_src,
                    resources_src,
                    archive_src,
                ],
                default_extension: extension_src,
                templates: TemplateOrigins {
                    note: note_src,
                    daily: daily_src,
                    project: project_src,
                    area: area_src,
                    resource: resource_src,
                },
            },
        ))
    }
}

/// Embedded at compile time; single source of truth for the schema's
/// contents, checked into the repo at assets/tick.schema.json.
const SCHEMA_JSON: &str = include_str!("../assets/tick.schema.json");

/// The exact filename `#:schema` points at and that `init` writes,
/// documented in README.md's "Schema and autocomplete" section.
const SCHEMA_FILENAME: &str = ".tick.schema.json";

/// Renders `Config::default()` as the exact `.tick.toml` shape documented in
/// README.md's Configuration section — a leading `#:schema` directive
/// (config.md 006) followed by nested `[folders]`/`[defaults]`/
/// `[templates]` tables.
pub fn default_toml() -> String {
    let defaults = Config::default();
    let templates = &defaults.templates;
    format!(
        r#"#:schema ./.tick.schema.json

[folders]
inbox = "{}"
projects = "{}"
areas = "{}"
resources = "{}"
archive = "{}"

[defaults]
extension = "{}"

[templates]
note = """
{}"""

daily = """
{}"""

project = """
{}"""

area = """
{}"""

resource = """
{}"""
"#,
        defaults.category_dirs[0],
        defaults.category_dirs[1],
        defaults.category_dirs[2],
        defaults.category_dirs[3],
        defaults.category_dirs[4],
        defaults.default_extension,
        templates.note,
        templates.daily,
        templates.project,
        templates.area,
        templates.resource,
    )
}

/// Writes `default_toml()` to `path`, alongside a sibling
/// `.tick.schema.json` that the `#:schema` directive in `default_toml()`
/// refers to. Errors with `AlreadyExists` (and leaves `path` untouched) if
/// a file is already there, rather than overwriting a user's
/// customizations.
///
/// The schema file is written first: if that write fails, `init` has
/// created nothing the caller needs to clean up, and a retry hits the same
/// state. Writing `.tick.toml` first would risk leaving behind a config
/// referencing a schema that was never written, with no way to retry the
/// schema write alone (the second call would fail `AlreadyExists` first).
pub fn init(path: &Path) -> Result<(), ConfigError> {
    if path.exists() {
        return Err(ConfigError::AlreadyExists {
            path: path.display().to_string(),
        });
    }
    let schema_path = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(SCHEMA_FILENAME);
    fs::write(&schema_path, SCHEMA_JSON).map_err(|source| ConfigError::Write {
        path: schema_path.display().to_string(),
        source,
    })?;
    fs::write(path, default_toml()).map_err(|source| ConfigError::Write {
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn merge_neither_set_yields_default() {
        let (value, source) = merge("default", None, None);

        assert_eq!(value, "default");
        assert_eq!(source, Source::Default);
    }

    #[test]
    fn merge_only_user_set_yields_user() {
        let (value, source) = merge("default", Some("user"), None);

        assert_eq!(value, "user");
        assert_eq!(source, Source::User);
    }

    #[test]
    fn merge_only_local_set_yields_local() {
        let (value, source) = merge("default", None, Some("local"));

        assert_eq!(value, "local");
        assert_eq!(source, Source::Local);
    }

    #[test]
    fn merge_both_set_yields_local_overrides_user() {
        let (value, source) = merge("default", Some("user"), Some("local"));

        assert_eq!(value, "local");
        assert_eq!(source, Source::LocalOverridesUser);
    }

    #[test]
    fn resolve_neither_file_present_returns_default_with_all_default_origins() {
        let dir = tempdir().unwrap();
        let local_path = dir.path().join(".tick.toml");

        let (config, origins) = Config::resolve(&local_path, None).unwrap();

        assert_eq!(config, Config::default());
        assert!(origins.category_dirs.iter().all(|s| *s == Source::Default));
        assert_eq!(origins.default_extension, Source::Default);
        assert_eq!(origins.templates.note, Source::Default);
        assert_eq!(origins.templates.daily, Source::Default);
        assert_eq!(origins.templates.project, Source::Default);
        assert_eq!(origins.templates.area, Source::Default);
        assert_eq!(origins.templates.resource, Source::Default);
    }

    #[test]
    fn resolve_only_local_overrides_one_key() {
        let dir = tempdir().unwrap();
        let local_path = dir.path().join(".tick.toml");
        fs::write(
            &local_path,
            r#"
            [folders]
            inbox = "Inbox"
            "#,
        )
        .unwrap();

        let (config, origins) = Config::resolve(&local_path, None).unwrap();

        assert_eq!(config.category_dirs[0], "Inbox");
        assert_eq!(origins.category_dirs[0], Source::Local);
        assert_eq!(origins.category_dirs[1], Source::Default);
        assert_eq!(origins.default_extension, Source::Default);
    }

    #[test]
    fn resolve_only_user_overrides_one_key() {
        let dir = tempdir().unwrap();
        let local_path = dir.path().join(".tick.toml");
        let home_path = dir.path().join("home.tick.toml");
        fs::write(
            &home_path,
            r#"
            [templates]
            note = "user note template"
            "#,
        )
        .unwrap();

        let (config, origins) = Config::resolve(&local_path, Some(&home_path)).unwrap();

        assert_eq!(config.templates.note, "user note template");
        assert_eq!(origins.templates.note, Source::User);
        assert_eq!(origins.templates.daily, Source::Default);
        assert_eq!(origins.default_extension, Source::Default);
    }

    #[test]
    fn resolve_user_and_local_set_disjoint_keys() {
        let dir = tempdir().unwrap();
        let local_path = dir.path().join(".tick.toml");
        let home_path = dir.path().join("home.tick.toml");
        fs::write(
            &local_path,
            r#"
            [folders]
            inbox = "Inbox"
            "#,
        )
        .unwrap();
        fs::write(
            &home_path,
            r#"
            [templates]
            daily = "user daily template"
            "#,
        )
        .unwrap();

        let (config, origins) = Config::resolve(&local_path, Some(&home_path)).unwrap();

        assert_eq!(config.category_dirs[0], "Inbox");
        assert_eq!(origins.category_dirs[0], Source::Local);
        assert_eq!(config.templates.daily, "user daily template");
        assert_eq!(origins.templates.daily, Source::User);
    }

    #[test]
    fn resolve_user_and_local_set_same_key_local_wins() {
        let dir = tempdir().unwrap();
        let local_path = dir.path().join(".tick.toml");
        let home_path = dir.path().join("home.tick.toml");
        fs::write(
            &local_path,
            r#"
            [templates]
            daily = "local daily template"
            "#,
        )
        .unwrap();
        fs::write(
            &home_path,
            r#"
            [templates]
            daily = "user daily template"
            "#,
        )
        .unwrap();

        let (config, origins) = Config::resolve(&local_path, Some(&home_path)).unwrap();

        assert_eq!(config.templates.daily, "local daily template");
        assert_eq!(origins.templates.daily, Source::LocalOverridesUser);
    }

    #[test]
    fn resolve_parses_nested_toml_shape_matching_readme() {
        let dir = tempdir().unwrap();
        let local_path = dir.path().join(".tick.toml");
        fs::write(
            &local_path,
            r#"
            [folders]
            inbox = "Inbox"
            projects = "Projects"
            areas = "Areas"
            resources = "Resources"
            archive = "Archive"

            [defaults]
            extension = "txt"

            [templates]
            note = "note template"
            daily = "daily template"
            project = "project template"
            area = "area template"
            resource = "resource template"
            "#,
        )
        .unwrap();

        let (config, _origins) = Config::resolve(&local_path, None).unwrap();

        assert_eq!(
            config.category_dirs,
            ["Inbox", "Projects", "Areas", "Resources", "Archive"]
        );
        assert_eq!(config.default_extension, "txt");
        assert_eq!(config.templates.note, "note template");
        assert_eq!(config.templates.daily, "daily template");
        assert_eq!(config.templates.project, "project template");
        assert_eq!(config.templates.area, "area template");
        assert_eq!(config.templates.resource, "resource template");
    }

    #[test]
    fn resolve_ignores_legacy_flat_toml_shape() {
        let dir = tempdir().unwrap();
        let local_path = dir.path().join(".tick.toml");
        fs::write(
            &local_path,
            r#"
            default_extension = "txt"

            [category_dirs]
            inbox = "Inbox"
            "#,
        )
        .unwrap();

        let (config, origins) = Config::resolve(&local_path, None).unwrap();

        assert_eq!(config, Config::default());
        assert_eq!(origins.default_extension, Source::Default);
        assert_eq!(origins.category_dirs[0], Source::Default);
    }

    #[test]
    fn render_fills_date_and_title_but_leaves_cursor_marker() {
        let template = "---\nlast_updated: {{date}}\n---\n# {{cursor}}{{title}}\n";

        let rendered = render(template, "", "2026-07-03", "14:32", "uuid-value");

        assert_eq!(
            rendered,
            "---\nlast_updated: 2026-07-03\n---\n# {{cursor}}\n"
        );
    }

    #[test]
    fn render_fills_time() {
        let template = "captured at {{time}}";

        let rendered = render(template, "", "2026-07-03", "14:32", "uuid-value");

        assert_eq!(rendered, "captured at 14:32");
    }

    #[test]
    fn render_fills_uuid() {
        let template = "id: {{uuid}}";

        let rendered = render(
            template,
            "",
            "2026-07-03",
            "14:32",
            "f47ac10b-58cc-4372-a567-0e02b2c3d479",
        );

        assert_eq!(rendered, "id: f47ac10b-58cc-4372-a567-0e02b2c3d479");
    }

    #[test]
    fn render_fills_all_markers_together_leaving_cursor_marker() {
        let template =
            "date={{date}} title={{title}} time={{time}} uuid={{uuid}} cursor={{cursor}}";

        let rendered = render(
            template,
            "My Title",
            "2026-07-03",
            "14:32",
            "f47ac10b-58cc-4372-a567-0e02b2c3d479",
        );

        assert_eq!(
            rendered,
            "date=2026-07-03 title=My Title time=14:32 uuid=f47ac10b-58cc-4372-a567-0e02b2c3d479 cursor={{cursor}}"
        );
    }

    #[test]
    fn for_kind_maps_to_matching_template() {
        use crate::category::Kind;

        let templates = Templates::default();

        assert_eq!(templates.for_kind(Kind::Inbox), templates.note);
        assert_eq!(templates.for_kind(Kind::Daily), templates.daily);
        assert_eq!(templates.for_kind(Kind::Project), templates.project);
        assert_eq!(templates.for_kind(Kind::Area), templates.area);
        assert_eq!(templates.for_kind(Kind::Resource), templates.resource);
    }

    #[test]
    fn daily_template_default_matches_readme() {
        let templates = Templates::default();

        assert!(templates.daily.contains("## Tasks"));
        assert!(templates.daily.contains("## Notes"));
    }

    #[test]
    fn empty_file_returns_default_values() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".tick.toml");
        fs::write(&path, "").unwrap();

        let (config, _origins) = Config::resolve(&path, None).unwrap();

        assert_eq!(config, Config::default());
    }

    #[test]
    fn default_toml_round_trips_to_config_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".tick.toml");
        fs::write(&path, default_toml()).unwrap();

        let (config, _origins) = Config::resolve(&path, None).unwrap();

        assert_eq!(config, Config::default());
    }

    #[test]
    fn default_toml_starts_with_schema_directive() {
        assert!(default_toml().starts_with("#:schema ./.tick.schema.json\n"));
    }

    #[test]
    fn schema_json_is_valid_json_with_expected_top_level_keys() {
        let parsed: serde_json::Value = serde_json::from_str(SCHEMA_JSON).unwrap();
        let properties = parsed.get("properties").unwrap();
        assert!(properties.get("folders").is_some());
        assert!(properties.get("defaults").is_some());
        assert!(properties.get("templates").is_some());
    }

    #[test]
    fn default_toml_matches_readme_documented_shape() {
        let toml = default_toml();

        // Nested tables, not the legacy flat shape.
        assert!(toml.contains("[folders]\ninbox = \"0-Inbox\""));
        assert!(toml.contains("[defaults]\nextension = \"md\""));
        assert!(toml.contains("[templates]\nnote = \"\"\"\n"));

        // Templates are triple-quoted multi-line strings, not single-line
        // escaped ones (README documents them as `"""..."""` blocks).
        assert!(toml.contains(
            "note = \"\"\"\n---\nlast_updated: {{date}}\n---\n# {{cursor}}{{title}}\n\"\"\""
        ));
        assert!(!toml.contains("note = \"---\\n"));
    }

    #[test]
    fn init_creates_file_when_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".tick.toml");

        init(&path).unwrap();

        assert!(path.exists());
        let (config, _origins) = Config::resolve(&path, None).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn init_writes_schema_file_alongside_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".tick.toml");

        init(&path).unwrap();

        let schema_path = dir.path().join(".tick.schema.json");
        assert!(schema_path.exists());
        assert_eq!(fs::read_to_string(&schema_path).unwrap(), SCHEMA_JSON);
    }

    #[test]
    fn init_refuses_when_file_already_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".tick.toml");
        fs::write(&path, "custom content").unwrap();

        let err = init(&path).unwrap_err();

        assert!(matches!(err, ConfigError::AlreadyExists { .. }));
        assert_eq!(fs::read_to_string(&path).unwrap(), "custom content");
        assert!(!dir.path().join(".tick.schema.json").exists());
    }
}

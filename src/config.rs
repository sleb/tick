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
    pub project: String,
    pub area: String,
    pub resource: String,
}

impl Default for Templates {
    fn default() -> Self {
        Self {
            note: "---\nlast_updated: {{date}}\n---\n# {{cursor}}{{title}}\n".to_string(),
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
    /// Maps a category to the template used when creating one of its
    /// items. Panics on `Category::Archive` — `items::create` (the only
    /// caller that renders a template) is never called with that variant,
    /// since items only arrive in `Archive` via `items::mv`.
    pub fn for_category(&self, category: crate::category::Category) -> &str {
        use crate::category::Category;
        match category {
            Category::Inbox => &self.note,
            Category::Project => &self.project,
            Category::Area => &self.area,
            Category::Resource => &self.resource,
            Category::Archive => panic!("Archive has no template; items arrive there via mv"),
        }
    }
}

/// Fills in `{{date}}` and `{{title}}` in `template`. Leaves `{{cursor}}`
/// untouched — interpreting that marker (positioning the editor's cursor,
/// then stripping it) is `Editor`'s job, not the renderer's.
pub fn render(template: &str, title: &str, date: &str) -> String {
    template
        .replace("{{date}}", date)
        .replace("{{title}}", title)
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
}

#[derive(Debug, Default, Deserialize)]
struct TomlConfig {
    default_extension: Option<String>,
    category_dirs: Option<TomlCategoryDirs>,
}

#[derive(Debug, Default, Deserialize)]
struct TomlCategoryDirs {
    inbox: Option<String>,
    project: Option<String>,
    area: Option<String>,
    resource: Option<String>,
    archive: Option<String>,
}

impl Config {
    /// Reads `.tick.toml` from `path` if it exists, merging any present
    /// fields over [`Config::default`]. Returns the default untouched if
    /// `path` doesn't exist.
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let raw = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        let parsed: TomlConfig = toml::from_str(&raw).map_err(|source| ConfigError::Parse {
            path: path.display().to_string(),
            source,
        })?;

        let mut config = Config::default();
        if let Some(ext) = parsed.default_extension {
            config.default_extension = ext;
        }
        if let Some(dirs) = parsed.category_dirs {
            // Order matches `Category as usize` (Inbox, Project, Area, Resource, Archive).
            let overrides = [
                dirs.inbox,
                dirs.project,
                dirs.area,
                dirs.resource,
                dirs.archive,
            ];
            for (i, value) in overrides.into_iter().enumerate() {
                if let Some(value) = value {
                    config.category_dirs[i] = value;
                }
            }
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn missing_file_returns_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".tick.toml");

        let config = Config::load(&path).unwrap();

        assert_eq!(config, Config::default());
    }

    #[test]
    fn present_file_merges_over_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".tick.toml");
        fs::write(
            &path,
            r#"
            default_extension = "txt"

            [category_dirs]
            inbox = "Inbox"
            "#,
        )
        .unwrap();

        let config = Config::load(&path).unwrap();

        assert_eq!(config.default_extension, "txt");
        assert_eq!(config.category_dirs[0], "Inbox");
        assert_eq!(config.category_dirs[1], "1-Projects");
    }

    #[test]
    fn render_fills_date_and_title_but_leaves_cursor_marker() {
        let template = "---\nlast_updated: {{date}}\n---\n# {{cursor}}{{title}}\n";

        let rendered = render(template, "", "2026-07-03");

        assert_eq!(
            rendered,
            "---\nlast_updated: 2026-07-03\n---\n# {{cursor}}\n"
        );
    }

    #[test]
    fn for_category_maps_to_matching_template() {
        use crate::category::Category;

        let templates = Templates::default();

        assert_eq!(templates.for_category(Category::Inbox), templates.note);
        assert_eq!(templates.for_category(Category::Project), templates.project);
        assert_eq!(templates.for_category(Category::Area), templates.area);
        assert_eq!(
            templates.for_category(Category::Resource),
            templates.resource
        );
    }

    #[test]
    #[should_panic]
    fn for_category_panics_on_archive() {
        use crate::category::Category;

        Templates::default().for_category(Category::Archive);
    }

    #[test]
    fn empty_file_returns_default_values() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".tick.toml");
        fs::write(&path, "").unwrap();

        let config = Config::load(&path).unwrap();

        assert_eq!(config, Config::default());
    }
}

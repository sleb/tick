use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::Local;
use thiserror::Error;

use crate::category::Category;
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
    fn choose(&mut self, prompt: &str, options: &[&str]) -> Result<char, UiError>;
}

pub struct TerminalUi;

impl Ui for TerminalUi {
    fn confirm(&mut self, prompt: &str, default: &str) -> Result<String, UiError> {
        print!("{prompt} [{default}] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            Ok(default.to_string())
        } else {
            Ok(trimmed.to_string())
        }
    }

    fn choose(&mut self, prompt: &str, options: &[&str]) -> Result<char, UiError> {
        loop {
            print!("{prompt} [{}] ", options.join("/"));
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let trimmed = input.trim().to_lowercase();
            if let Some(choice) = trimmed.chars().next()
                && options
                    .iter()
                    .any(|o| o.to_lowercase() == choice.to_string())
            {
                return Ok(choice);
            }
        }
    }
}

pub fn run_new(
    ws: &Workspace,
    editor: &dyn Editor,
    ui: &mut dyn Ui,
    category: Category,
    filename: Option<String>,
) -> anyhow::Result<PathBuf> {
    let path = match filename {
        Some(name) => {
            let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
            let rendered =
                config::render(ws.config.templates.for_category(category), &name, &today)
                    .replace("{{cursor}}", "");
            items::create(ws, category, &name, &rendered)?
        }
        None => {
            let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
            let seed = config::render(&ws.config.templates.note, "", &today);
            let (content, suggested) = editor.capture(&seed)?;
            let default = format!("{suggested}.{}", ws.config.default_extension);
            let chosen = ui.confirm(&format!("Create \"{default}\"?"), &default)?;
            items::create(ws, Category::Inbox, &chosen, &content)?
        }
    };
    Ok(path)
}

pub fn run_init(cwd: &Path, name: Option<&str>) -> anyhow::Result<String> {
    let (target, display) = match name {
        Some(n) => (cwd.join(n), format!("./{n}")),
        None => (cwd.to_path_buf(), ".".to_string()),
    };

    let report = workspace::init(&target)?;

    Ok(match report.created.len() {
        5 => format!("Created PARA system in {display}"),
        0 => format!("PARA system in {display} is already complete; no changes made"),
        _ => format!("Created {} in {display}", report.created.join(", ")),
    })
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
    }

    struct FakeUi {
        confirm_response: String,
    }

    impl Ui for FakeUi {
        fn confirm(&mut self, _prompt: &str, _default: &str) -> Result<String, UiError> {
            Ok(self.confirm_response.clone())
        }

        fn choose(&mut self, _prompt: &str, _options: &[&str]) -> Result<char, UiError> {
            unimplemented!("not exercised by `new` story 001")
        }
    }

    fn workspace(root: &std::path::Path) -> Workspace {
        Workspace {
            root: root.to_path_buf(),
            config: Config::default(),
        }
    }

    struct PanicEditor;

    impl Editor for PanicEditor {
        fn capture(&self, _seed: &str) -> Result<(String, String), EditorError> {
            panic!("editor should not be invoked when a filename is given")
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

        let path = run_new(&ws, &editor, &mut ui, Category::Inbox, None).unwrap();

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

        let path = run_new(&ws, &editor, &mut ui, Category::Inbox, None).unwrap();

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

        let path = run_new(&ws, &editor, &mut ui, Category::Inbox, None).unwrap();

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
        }

        let dir = tempdir().unwrap();
        let ws = workspace(dir.path());
        let editor = RecordingEditor {
            seen_seed: RefCell::new(String::new()),
        };
        let mut ui = FakeUi {
            confirm_response: "title.md".to_string(),
        };

        run_new(&ws, &editor, &mut ui, Category::Inbox, None).unwrap();

        let seed = editor.seen_seed.borrow();
        assert!(seed.contains("{{cursor}}"));
        assert!(!seed.contains("{{title}}"));
        assert!(!seed.contains("{{date}}"));
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
            Category::Inbox,
            Some("my-file".to_string()),
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
            Category::Project,
            Some("website-redesign".to_string()),
        )
        .unwrap();

        assert_eq!(
            path,
            dir.path().join("1-Projects/website-redesign/index.md")
        );
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# website-redesign"));
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
            Category::Area,
            Some("health".to_string()),
        )
        .unwrap();

        assert_eq!(path, dir.path().join("2-Areas/health/index.md"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# health"));
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
            Category::Resource,
            Some("recipe-ideas".to_string()),
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
            Category::Inbox,
            Some("my-file".to_string()),
        )
        .unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        assert!(content.contains(&format!("last_updated: {today}")));
    }

    #[test]
    fn run_init_bare_full_create() {
        let dir = tempdir().unwrap();

        let message = run_init(dir.path(), None).unwrap();

        assert_eq!(message, "Created PARA system in .");
    }

    #[test]
    fn run_init_named_full_create() {
        let dir = tempdir().unwrap();

        let message = run_init(dir.path(), Some("my-para")).unwrap();

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

        let message = run_init(dir.path(), None).unwrap();

        assert_eq!(
            message,
            "PARA system in . is already complete; no changes made"
        );
    }

    #[test]
    fn run_init_partial_fill_in() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("0-Inbox")).unwrap();

        let message = run_init(dir.path(), None).unwrap();

        assert_eq!(
            message,
            "Created 1-Projects, 2-Areas, 3-Resources, 4-Archive in ."
        );
    }

    #[test]
    fn run_init_bare_tolerates_unrelated_contents() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "hello").unwrap();

        let message = run_init(dir.path(), None).unwrap();

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

        let err = run_init(dir.path(), Some("existing-file")).unwrap_err();

        assert!(err.to_string().contains("existing-file"));
        assert!(err.to_string().contains("already exists"));
    }
}

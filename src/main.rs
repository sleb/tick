use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Args, CommandFactory, Parser, Subcommand};

use tick::category::{Category, Kind};
use tick::cli::{self, TerminalUi};
use tick::editor::RealEditor;
use tick::workspace::Workspace;

#[derive(Parser)]
#[command(name = "tk")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, PartialEq, Subcommand)]
enum Commands {
    /// Capture a new note.
    New {
        filename: Option<String>,
        #[command(flatten)]
        category: NewCategory,
    },
    /// Create (or open) today's daily note in the Inbox.
    Daily,
    /// Scaffold a PARA system.
    Init { name: Option<String> },
    /// View or manage the effective config.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// List items in a category.
    #[command(alias = "ls")]
    List {
        category: ListCategory,
        filter: Option<String>,
    },
    /// Print a shell completion script for `tk` to stdout.
    Completions { shell: CompletionShell },
    /// Print a per-category summary of the PARA system.
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Powershell,
}

impl From<CompletionShell> for clap_complete::Shell {
    fn from(shell: CompletionShell) -> Self {
        match shell {
            CompletionShell::Bash => clap_complete::Shell::Bash,
            CompletionShell::Zsh => clap_complete::Shell::Zsh,
            CompletionShell::Fish => clap_complete::Shell::Fish,
            CompletionShell::Powershell => clap_complete::Shell::PowerShell,
        }
    }
}

/// Renders `shell`'s completion script for the `tk` CLI into a byte buffer.
fn render_completions(shell: CompletionShell) -> Vec<u8> {
    let mut buf = Vec::new();
    clap_complete::generate(
        clap_complete::Shell::from(shell),
        &mut Cli::command(),
        "tk",
        &mut buf,
    );
    buf
}

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
enum ListCategory {
    Project,
    Area,
    Resource,
    Inbox,
    Archive,
}

impl From<ListCategory> for Category {
    fn from(category: ListCategory) -> Self {
        match category {
            ListCategory::Project => Category::Project,
            ListCategory::Area => Category::Area,
            ListCategory::Resource => Category::Resource,
            ListCategory::Inbox => Category::Inbox,
            ListCategory::Archive => Category::Archive,
        }
    }
}

#[derive(Debug, PartialEq, Subcommand)]
enum ConfigAction {
    /// Write a new `.tick.toml` (or `~/.tick.toml` with `-g`) populated
    /// with the built-in defaults.
    Init {
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
    /// Open `.tick.toml` (or `~/.tick.toml` with `-g`) in `$EDITOR`,
    /// creating it with the default config first if it doesn't exist yet.
    Edit {
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Args)]
#[group(multiple = false)]
struct NewCategory {
    /// Scaffold a project directory instead of an Inbox file.
    #[arg(long)]
    project: bool,
    /// Scaffold an area directory instead of an Inbox file.
    #[arg(long)]
    area: bool,
    /// Create a flat resource file instead of an Inbox file.
    #[arg(long)]
    resource: bool,
    /// Create (or open) today's daily note instead of an Inbox file.
    #[arg(long, conflicts_with = "filename")]
    daily: bool,
}

impl NewCategory {
    fn into_kind(self) -> Kind {
        if self.project {
            Kind::Project
        } else if self.area {
            Kind::Area
        } else if self.resource {
            Kind::Resource
        } else if self.daily {
            Kind::Daily
        } else {
            Kind::Inbox
        }
    }
}

/// Computes the local-vs-global config target: the path to write/open, and
/// its human-readable display form (`"./.tick.toml"` or `"~/.tick.toml"`).
fn config_target(cwd: &Path, global: bool) -> anyhow::Result<(PathBuf, String)> {
    Ok(if global {
        let home = env::var_os("HOME").context("$HOME is not set")?;
        (
            PathBuf::from(&home).join(".tick.toml"),
            "~/.tick.toml".to_string(),
        )
    } else {
        (cwd.join(".tick.toml"), "./.tick.toml".to_string())
    })
}

fn run_daily_command(ws: &Workspace) -> anyhow::Result<()> {
    if cli::daily_note_exists(ws) {
        println!("Opening $EDITOR...");
    }
    let editor = RealEditor;
    if let cli::DailyOutcome::Created(path) = cli::run_daily(ws, &editor)? {
        println!("Created {}", path.display());
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let cwd = env::current_dir().context("failed to determine current directory")?;
    let home_config = env::var_os("HOME").map(|home| PathBuf::from(home).join(".tick.toml"));

    match cli.command {
        Commands::Init { name } => {
            let message = cli::run_init(&cwd, name.as_deref())?;
            println!("{message}");
        }
        Commands::Daily => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            run_daily_command(&ws)?;
        }
        Commands::New {
            filename: _,
            category,
        } if category.into_kind() == Kind::Daily => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            run_daily_command(&ws)?;
        }
        Commands::New { filename, category } => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            if filename.is_none() {
                println!("Opening $EDITOR...");
            }
            let editor = RealEditor;
            let mut ui = TerminalUi;
            let path = cli::run_new(&ws, &editor, &mut ui, category.into_kind(), filename)?;
            println!("Created {}", path.display());
        }
        Commands::Config {
            action: ConfigAction::Init { global },
        } => {
            let (path, display) = config_target(&cwd, global)?;
            let message = cli::run_config_init(&path, &display)?;
            println!("{message}");
        }
        Commands::Config {
            action: ConfigAction::Edit { global },
        } => {
            let (path, display) = config_target(&cwd, global)?;
            let editor = RealEditor;
            if cli::run_config_edit(&path, &editor)? {
                println!("Created {display}");
            }
            println!("Opening $EDITOR...");
        }
        Commands::List { category, filter } => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            let output = cli::run_list(&ws, category.into(), filter.as_deref())?;
            println!("{output}");
        }
        Commands::Completions { shell } => {
            io::stdout().write_all(&render_completions(shell))?;
        }
        Commands::Status => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            let output = cli::run_status(&ws)?;
            println!("{output}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_new_with_filename() {
        let cli = Cli::parse_from(["tk", "new", "my-file"]);

        assert_eq!(
            cli.command,
            Commands::New {
                filename: Some("my-file".to_string()),
                category: NewCategory::default(),
            }
        );
    }

    #[test]
    fn parses_new_project() {
        let cli = Cli::parse_from(["tk", "new", "--project", "website-redesign"]);

        assert_eq!(
            cli.command,
            Commands::New {
                filename: Some("website-redesign".to_string()),
                category: NewCategory {
                    project: true,
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn parses_new_area() {
        let cli = Cli::parse_from(["tk", "new", "--area", "health"]);

        assert_eq!(
            cli.command,
            Commands::New {
                filename: Some("health".to_string()),
                category: NewCategory {
                    area: true,
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn parses_new_resource() {
        let cli = Cli::parse_from(["tk", "new", "--resource", "recipe-ideas"]);

        assert_eq!(
            cli.command,
            Commands::New {
                filename: Some("recipe-ideas".to_string()),
                category: NewCategory {
                    resource: true,
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn rejects_conflicting_category_flags() {
        let result = Cli::try_parse_from(["tk", "new", "--project", "--area", "x"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_new_daily() {
        let cli = Cli::parse_from(["tk", "new", "--daily"]);

        assert_eq!(
            cli.command,
            Commands::New {
                filename: None,
                category: NewCategory {
                    daily: true,
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn rejects_new_daily_with_filename() {
        let result = Cli::try_parse_from(["tk", "new", "--daily", "x"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_new_daily_with_project() {
        let result = Cli::try_parse_from(["tk", "new", "--daily", "--project"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_daily() {
        let cli = Cli::parse_from(["tk", "daily"]);

        assert_eq!(cli.command, Commands::Daily);
    }

    #[test]
    fn parses_status() {
        let cli = Cli::parse_from(["tk", "status"]);

        assert_eq!(cli.command, Commands::Status);
    }

    #[test]
    fn rejects_daily_with_filename() {
        let result = Cli::try_parse_from(["tk", "daily", "x"]);

        assert!(result.is_err());
    }

    #[test]
    fn into_kind_defaults_to_inbox() {
        assert_eq!(NewCategory::default().into_kind(), Kind::Inbox);
    }

    #[test]
    fn into_kind_maps_every_flag() {
        assert_eq!(
            NewCategory {
                project: true,
                ..Default::default()
            }
            .into_kind(),
            Kind::Project
        );
        assert_eq!(
            NewCategory {
                area: true,
                ..Default::default()
            }
            .into_kind(),
            Kind::Area
        );
        assert_eq!(
            NewCategory {
                resource: true,
                ..Default::default()
            }
            .into_kind(),
            Kind::Resource
        );
        assert_eq!(
            NewCategory {
                daily: true,
                ..Default::default()
            }
            .into_kind(),
            Kind::Daily
        );
    }

    #[test]
    fn parses_init_with_name() {
        let cli = Cli::parse_from(["tk", "init", "my-para"]);

        assert_eq!(
            cli.command,
            Commands::Init {
                name: Some("my-para".to_string())
            }
        );
    }

    #[test]
    fn parses_init_without_name() {
        let cli = Cli::parse_from(["tk", "init"]);

        assert_eq!(cli.command, Commands::Init { name: None });
    }

    #[test]
    fn parses_config_init_with_no_flag() {
        let cli = Cli::parse_from(["tk", "config", "init"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: ConfigAction::Init { global: false }
            }
        );
    }

    #[test]
    fn parses_config_init_global_short_flag() {
        let cli = Cli::parse_from(["tk", "config", "init", "-g"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: ConfigAction::Init { global: true }
            }
        );
    }

    #[test]
    fn parses_config_init_global_long_flag() {
        let cli = Cli::parse_from(["tk", "config", "init", "--global"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: ConfigAction::Init { global: true }
            }
        );
    }

    #[test]
    fn parses_config_edit_with_no_flag() {
        let cli = Cli::parse_from(["tk", "config", "edit"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: ConfigAction::Edit { global: false }
            }
        );
    }

    #[test]
    fn parses_config_edit_global_short_flag() {
        let cli = Cli::parse_from(["tk", "config", "edit", "-g"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: ConfigAction::Edit { global: true }
            }
        );
    }

    #[test]
    fn parses_config_edit_global_long_flag() {
        let cli = Cli::parse_from(["tk", "config", "edit", "--global"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: ConfigAction::Edit { global: true }
            }
        );
    }

    #[test]
    fn rejects_config_with_no_subcommand() {
        let result = Cli::try_parse_from(["tk", "config"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_list_project() {
        let cli = Cli::parse_from(["tk", "list", "project"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Project,
                filter: None
            }
        );
    }

    #[test]
    fn parses_list_project_with_filter() {
        let cli = Cli::parse_from(["tk", "list", "project", "web"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Project,
                filter: Some("web".into())
            }
        );
    }

    #[test]
    fn parses_list_area() {
        let cli = Cli::parse_from(["tk", "list", "area"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Area,
                filter: None
            }
        );
    }

    #[test]
    fn parses_list_resource() {
        let cli = Cli::parse_from(["tk", "list", "resource"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Resource,
                filter: None
            }
        );
    }

    #[test]
    fn parses_list_inbox() {
        let cli = Cli::parse_from(["tk", "list", "inbox"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Inbox,
                filter: None
            }
        );
    }

    #[test]
    fn parses_list_archive() {
        let cli = Cli::parse_from(["tk", "list", "archive"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Archive,
                filter: None
            }
        );
    }

    #[test]
    fn parses_completions_bash() {
        let cli = Cli::parse_from(["tk", "completions", "bash"]);

        assert_eq!(
            cli.command,
            Commands::Completions {
                shell: CompletionShell::Bash
            }
        );
    }

    #[test]
    fn parses_completions_zsh() {
        let cli = Cli::parse_from(["tk", "completions", "zsh"]);

        assert_eq!(
            cli.command,
            Commands::Completions {
                shell: CompletionShell::Zsh
            }
        );
    }

    #[test]
    fn parses_completions_fish() {
        let cli = Cli::parse_from(["tk", "completions", "fish"]);

        assert_eq!(
            cli.command,
            Commands::Completions {
                shell: CompletionShell::Fish
            }
        );
    }

    #[test]
    fn parses_completions_powershell() {
        let cli = Cli::parse_from(["tk", "completions", "powershell"]);

        assert_eq!(
            cli.command,
            Commands::Completions {
                shell: CompletionShell::Powershell
            }
        );
    }

    #[test]
    fn rejects_unsupported_completions_shell() {
        let result = Cli::try_parse_from(["tk", "completions", "tcsh"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_completions_shell() {
        let result = Cli::try_parse_from(["tk", "completions"]);

        assert!(result.is_err());
    }

    #[test]
    fn renders_non_empty_completions_for_every_shell() {
        for shell in [
            CompletionShell::Bash,
            CompletionShell::Zsh,
            CompletionShell::Fish,
            CompletionShell::Powershell,
        ] {
            assert!(!render_completions(shell).is_empty());
        }
    }

    #[test]
    fn completions_cover_every_top_level_command() {
        let script = render_completions(CompletionShell::Bash);
        let script = String::from_utf8(script).unwrap();

        for command in ["init", "new", "daily", "list", "config", "completions"] {
            assert!(
                script.contains(command),
                "expected script to contain {command}"
            );
        }
    }
}

use std::env;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use clap_complete::env::{Bash, EnvCompleter, Fish, Powershell, Zsh};

use tick::category::{Category, Kind};
use tick::cli::{self, TerminalUi};
use tick::editor::RealEditor;
use tick::review;
use tick::workspace::Workspace;

#[derive(Parser)]
#[command(name = "tk", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, PartialEq, Subcommand)]
enum Commands {
    /// Capture a new note.
    #[command(after_help = "\
Examples:
  tk new                     Open $EDITOR and suggest a filename from its content
  tk new meeting-notes       Create ./0-Inbox/meeting-notes.md directly
  tk new --project apollo    Scaffold a new project directory")]
    New {
        /// Name of the file to create (extension added automatically).
        /// Omit to open $EDITOR and be prompted with a suggested name.
        filename: Option<String>,
        #[command(flatten)]
        category: NewCategory,
        /// Accept the suggested filename without prompting.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// Create (or open) today's daily note in the Inbox.
    Daily,
    /// Scaffold a PARA system.
    Init { name: Option<String> },
    /// View or manage the effective config.
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
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
    /// Relocate an item to a different category.
    #[command(
        alias = "mv",
        after_help = "\
Examples:
  tk move meeting-notes project   File an Inbox item as a project
  tk mv apollo archive            Archive a project (prompts for a summary)
  tk move apollo archive --yes    Archive it, accepting the suggested summary"
    )]
    Move {
        /// Name of the item to relocate, as shown by `tk list`.
        #[arg(add = ArgValueCompleter::new(complete_move_name))]
        name: String,
        /// Category to move the item into.
        target: MoveTarget,
        /// Accept the suggested archive summary without prompting.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// File an item away — sugar for `tk move <item> archive`.
    #[command(after_help = "\
Examples:
  tk archive apollo         Archive \"apollo\" (prompts for a summary)
  tk archive apollo --yes   Archive it, accepting the suggested summary")]
    Archive {
        /// Name of the item to archive, as shown by `tk list`.
        #[arg(add = ArgValueCompleter::new(complete_archive_name))]
        name: String,
        /// Accept the suggested archive summary without prompting.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// Restore an archived item to the category it was archived from.
    #[command(after_help = "\
Examples:
  tk unarchive Projects/apollo   Restore \"apollo\" to 1-Projects")]
    Unarchive {
        /// Qualified `<OriginCategory>/<name>` of the archived item, as
        /// shown by `tk list archive`.
        #[arg(add = ArgValueCompleter::new(complete_unarchive_name))]
        name: String,
    },
    /// Walk every project and area, prompting keep/archive/skip.
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
enum MoveTarget {
    Project,
    Area,
    Resource,
    Inbox,
    Archive,
}

impl From<MoveTarget> for Category {
    fn from(target: MoveTarget) -> Self {
        match target {
            MoveTarget::Project => Category::Project,
            MoveTarget::Area => Category::Area,
            MoveTarget::Resource => Category::Resource,
            MoveTarget::Inbox => Category::Inbox,
            MoveTarget::Archive => Category::Archive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Powershell,
}

/// Discovers a workspace from the current process's cwd/`$HOME`, applies
/// `names` to it, and filters to candidates matching `current`'s prefix —
/// the shared plumbing every `complete_*_name` function needs. Returns no
/// candidates (never errors, never blocks) if no workspace is found,
/// satisfying completions.md 004's "no PARA system" scenario.
fn complete_item_name(
    current: &OsStr,
    names: impl Fn(&Workspace) -> Vec<String>,
) -> Vec<CompletionCandidate> {
    let Some(current) = current.to_str() else {
        return vec![];
    };
    let Ok(cwd) = env::current_dir() else {
        return vec![];
    };
    let Ok(ws) = Workspace::discover(&cwd, home_tick_toml().as_deref()) else {
        return vec![];
    };
    names(&ws)
        .into_iter()
        .filter(|name| name.starts_with(current))
        .map(CompletionCandidate::new)
        .collect()
}

fn complete_move_name(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_item_name(current, |ws| {
        let mut names = cli::live_item_names(ws);
        names.extend(cli::archived_item_names(ws));
        names
    })
}

fn complete_archive_name(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_item_name(current, cli::live_item_names)
}

fn complete_unarchive_name(current: &OsStr) -> Vec<CompletionCandidate> {
    complete_item_name(current, cli::archived_item_names)
}

/// Writes the `clap_complete::env` registration snippet for `shell` — shell
/// glue that re-invokes `tk` (via `$PATH`) at completion time — rather than
/// a static per-command script.
fn render_completions(shell: CompletionShell) -> Vec<u8> {
    let mut buf = Vec::new();
    let completer: &dyn EnvCompleter = match shell {
        CompletionShell::Bash => &Bash,
        CompletionShell::Zsh => &Zsh,
        CompletionShell::Fish => &Fish,
        CompletionShell::Powershell => &Powershell,
    };
    completer
        .write_registration("COMPLETE", "tk", "tk", "tk", &mut buf)
        .expect("writing to an in-memory buffer never fails");
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

/// Resolves `~/.tick.toml`, or `None` if `$HOME` isn't set.
fn home_tick_toml() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".tick.toml"))
}

/// Computes the local-vs-global config target: the path to write/open, and
/// its human-readable display form (`"./.tick.toml"` or `"~/.tick.toml"`).
fn config_target(cwd: &Path, global: bool) -> anyhow::Result<(PathBuf, String)> {
    Ok(if global {
        let path = home_tick_toml().context("$HOME is not set")?;
        (path, "~/.tick.toml".to_string())
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
        println!("Next: tk list to see it, or tk status for an overview.");
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();

    env_logger::init();
    let cli = Cli::parse();

    let cwd = env::current_dir().context("failed to determine current directory")?;
    let home_config = home_tick_toml();

    match cli.command {
        Commands::Init { name } => {
            let message = cli::run_init(&cwd, name.as_deref(), home_config.as_deref())?;
            println!("{message}");
            match name.as_deref() {
                Some(n) => println!("Next: cd {n} && tk new to capture your first note."),
                None => println!("Next: tk new to capture your first note."),
            }
        }
        Commands::Daily => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            run_daily_command(&ws)?;
        }
        Commands::New {
            filename: _,
            category,
            yes: _,
        } if category.into_kind() == Kind::Daily => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            run_daily_command(&ws)?;
        }
        Commands::New {
            filename,
            category,
            yes,
        } => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            if filename.is_none() {
                println!("Opening $EDITOR...");
            }
            let editor = RealEditor;
            let mut ui = TerminalUi;
            let path = cli::run_new(&ws, &editor, &mut ui, category.into_kind(), filename, yes)?;
            println!("Created {}", path.display());
            println!("Next: tk list to see it, or tk status for an overview.");
        }
        Commands::Config {
            action: Some(ConfigAction::Init { global }),
        } => {
            let (path, display) = config_target(&cwd, global)?;
            let message = cli::run_config_init(&path, &display)?;
            println!("{message}");
        }
        Commands::Config {
            action: Some(ConfigAction::Edit { global }),
        } => {
            let (path, display) = config_target(&cwd, global)?;
            if !path.exists() {
                println!("Created {display}");
            }
            println!("Opening $EDITOR...");
            let editor = RealEditor;
            cli::run_config_edit(&path, &editor)?;
        }
        Commands::Config { action: None } => {
            let (path, _display) = config_target(&cwd, false)?;
            let (config, origins) = tick::config::Config::resolve(&path, home_config.as_deref())?;
            print!("{}", tick::config::render_effective(&config, &origins));
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
        Commands::Move { name, target, yes } => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            let mut ui = TerminalUi;
            let message = cli::run_move(&ws, &mut ui, &name, target.into(), yes)?;
            println!("{message}");
        }
        Commands::Archive { name, yes } => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            let mut ui = TerminalUi;
            let message = cli::run_move(&ws, &mut ui, &name, Category::Archive, yes)?;
            println!("{message}");
        }
        Commands::Unarchive { name } => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            let message = cli::run_unarchive(&ws, &name)?;
            println!("{message}");
        }
        Commands::Review => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            let mut ui = TerminalUi;
            review::run(&ws, &mut ui)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_new_with_filename() {
        let cli = Cli::parse_from(["tk", "new", "my-file"]);

        assert_eq!(
            cli.command,
            Commands::New {
                filename: Some("my-file".to_string()),
                category: NewCategory::default(),
                yes: false,
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
                yes: false,
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
                yes: false,
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
                yes: false,
            }
        );
    }

    #[test]
    fn parses_new_yes_flag() {
        let cli = Cli::parse_from(["tk", "new", "--yes"]);

        assert_eq!(
            cli.command,
            Commands::New {
                filename: None,
                category: NewCategory::default(),
                yes: true,
            }
        );
    }

    #[test]
    fn parses_new_yes_short_flag() {
        let cli = Cli::parse_from(["tk", "new", "-y"]);

        assert_eq!(
            cli.command,
            Commands::New {
                filename: None,
                category: NewCategory::default(),
                yes: true,
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
                yes: false,
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
    fn parses_move() {
        let cli = Cli::parse_from(["tk", "move", "my-file", "project"]);

        assert_eq!(
            cli.command,
            Commands::Move {
                name: "my-file".to_string(),
                target: MoveTarget::Project,
                yes: false,
            }
        );
    }

    #[test]
    fn parses_mv_alias() {
        let cli = Cli::parse_from(["tk", "mv", "my-file", "archive"]);

        assert_eq!(
            cli.command,
            Commands::Move {
                name: "my-file".to_string(),
                target: MoveTarget::Archive,
                yes: false,
            }
        );
    }

    #[test]
    fn parses_archive() {
        let cli = Cli::parse_from(["tk", "archive", "my-file"]);

        assert_eq!(
            cli.command,
            Commands::Archive {
                name: "my-file".to_string(),
                yes: false,
            }
        );
    }

    #[test]
    fn parses_archive_yes_flag() {
        let cli = Cli::parse_from(["tk", "archive", "my-file", "--yes"]);

        assert_eq!(
            cli.command,
            Commands::Archive {
                name: "my-file".to_string(),
                yes: true,
            }
        );
    }

    #[test]
    fn parses_move_yes_flag() {
        let cli = Cli::parse_from(["tk", "move", "my-file", "archive", "-y"]);

        assert_eq!(
            cli.command,
            Commands::Move {
                name: "my-file".to_string(),
                target: MoveTarget::Archive,
                yes: true,
            }
        );
    }

    #[test]
    fn rejects_archive_with_category_argument() {
        let result = Cli::try_parse_from(["tk", "archive", "my-file", "archive"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_unarchive() {
        let cli = Cli::parse_from(["tk", "unarchive", "Projects/my-file"]);

        assert_eq!(
            cli.command,
            Commands::Unarchive {
                name: "Projects/my-file".to_string(),
            }
        );
    }

    #[test]
    fn rejects_unarchive_with_category_argument() {
        let result = Cli::try_parse_from(["tk", "unarchive", "Projects/my-file", "project"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_review() {
        let cli = Cli::parse_from(["tk", "review"]);

        assert_eq!(cli.command, Commands::Review);
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
                action: Some(ConfigAction::Init { global: false })
            }
        );
    }

    #[test]
    fn parses_config_init_global_short_flag() {
        let cli = Cli::parse_from(["tk", "config", "init", "-g"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Init { global: true })
            }
        );
    }

    #[test]
    fn parses_config_init_global_long_flag() {
        let cli = Cli::parse_from(["tk", "config", "init", "--global"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Init { global: true })
            }
        );
    }

    #[test]
    fn parses_config_edit_with_no_flag() {
        let cli = Cli::parse_from(["tk", "config", "edit"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Edit { global: false })
            }
        );
    }

    #[test]
    fn parses_config_edit_global_short_flag() {
        let cli = Cli::parse_from(["tk", "config", "edit", "-g"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Edit { global: true })
            }
        );
    }

    #[test]
    fn parses_config_edit_global_long_flag() {
        let cli = Cli::parse_from(["tk", "config", "edit", "--global"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Edit { global: true })
            }
        );
    }

    #[test]
    fn parses_config_bare_as_action_none() {
        let cli = Cli::parse_from(["tk", "config"]);

        assert_eq!(cli.command, Commands::Config { action: None });
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
        let candidates = complete(&["tk", ""], 1);

        for command in [
            "init",
            "new",
            "daily",
            "list",
            "config",
            "move",
            "archive",
            "unarchive",
            "status",
            "review",
            "completions",
        ] {
            assert!(
                candidates.contains(&command.to_string()),
                "expected {command} among top-level completions, got {candidates:?}"
            );
        }
    }

    /// Drives `clap_complete::engine::complete()` against `Cli::command()`
    /// for `args`, completing the argument at `arg_index`, and returns the
    /// candidates' string values, excluding flag candidates (e.g.
    /// `--help`) — the dynamic engine always offers those alongside a
    /// positional's own value completions, but they're not part of what
    /// these tests are checking.
    fn complete(args: &[&str], arg_index: usize) -> Vec<String> {
        let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
        clap_complete::engine::complete(&mut Cli::command(), args, arg_index, None)
            .unwrap()
            .into_iter()
            .map(|c| c.get_value().to_string_lossy().into_owned())
            .filter(|value| !value.starts_with('-'))
            .collect()
    }

    /// Serializes tests that change the process's current directory —
    /// `env::current_dir` is process-global state, and `cargo test` runs
    /// tests in parallel within one process.
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Writes an empty `.tick.toml` marker at `root` (matching
    /// `workspace::init`'s discovery contract) and returns a `Workspace`
    /// rooted there with default config, for tests that need a real
    /// on-disk PARA system for `Workspace::discover` to find via cwd.
    fn init_workspace(root: &Path) -> Workspace {
        fs::write(root.join(".tick.toml"), "").unwrap();
        Workspace {
            root: root.to_path_buf(),
            config: tick::config::Config::default(),
        }
    }

    #[test]
    fn completes_a_live_items_bare_name() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        tick::items::create(&ws, tick::category::Category::Inbox, "my-file", "hello").unwrap();
        tick::items::create(
            &ws,
            tick::category::Category::Project,
            "website-redesign",
            "",
        )
        .unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["tk", "move", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        let mut candidates = candidates;
        candidates.sort();
        assert_eq!(candidates, vec!["my-file", "website-redesign"]);
    }

    #[test]
    fn completes_an_archived_items_qualified_name() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        let path =
            tick::items::create(&ws, tick::category::Category::Inbox, "meeting-notes", "").unwrap();
        tick::items::mv(
            &ws,
            tick::category::Category::Inbox,
            &path,
            "meeting-notes",
            tick::category::Category::Archive,
        )
        .unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["tk", "unarchive", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        assert_eq!(candidates, vec!["Inbox/meeting-notes"]);
    }

    #[test]
    fn archive_completion_excludes_archived_items() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        tick::items::create(&ws, tick::category::Category::Project, "apollo", "").unwrap();
        let path =
            tick::items::create(&ws, tick::category::Category::Inbox, "meeting-notes", "").unwrap();
        tick::items::mv(
            &ws,
            tick::category::Category::Inbox,
            &path,
            "meeting-notes",
            tick::category::Category::Archive,
        )
        .unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["tk", "archive", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        assert_eq!(candidates, vec!["apollo"]);
    }

    #[test]
    fn move_completion_includes_both_live_and_archived_items() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        tick::items::create(&ws, tick::category::Category::Project, "apollo", "").unwrap();
        let path =
            tick::items::create(&ws, tick::category::Category::Inbox, "meeting-notes", "").unwrap();
        tick::items::mv(
            &ws,
            tick::category::Category::Inbox,
            &path,
            "meeting-notes",
            tick::category::Category::Archive,
        )
        .unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let mut candidates = complete(&["tk", "move", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        candidates.sort();
        assert_eq!(candidates, vec!["Inbox/meeting-notes", "apollo"]);
    }

    #[test]
    fn completions_reflect_the_current_directorys_para_system() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let first = tempfile::tempdir().unwrap();
        let first_ws = init_workspace(first.path());
        tick::items::create(&first_ws, tick::category::Category::Inbox, "first-item", "").unwrap();
        let second = tempfile::tempdir().unwrap();
        let second_ws = init_workspace(second.path());
        tick::items::create(
            &second_ws,
            tick::category::Category::Inbox,
            "second-item",
            "",
        )
        .unwrap();

        env::set_current_dir(first.path()).unwrap();
        let first_candidates = complete(&["tk", "move", ""], 2);
        env::set_current_dir(second.path()).unwrap();
        let second_candidates = complete(&["tk", "move", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        assert_eq!(first_candidates, vec!["first-item"]);
        assert_eq!(second_candidates, vec!["second-item"]);
        assert_ne!(first_candidates, second_candidates);
    }

    #[test]
    fn no_para_system_yields_no_item_name_completions() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["tk", "move", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        assert!(candidates.is_empty());
    }
}

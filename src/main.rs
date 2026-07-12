use std::env;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use clap_complete::env::{Bash, EnvCompleter, Fish, Powershell, Zsh};

use ishi::category::{Category, Kind};
use ishi::cli::{self, TerminalUi};
use ishi::editor::RealEditor;
use ishi::review;
use ishi::workspace::Workspace;

#[derive(Parser)]
#[command(name = "ishi", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, PartialEq, Subcommand)]
enum Commands {
    /// Capture a new note.
    #[command(after_help = "\
Examples:
  ishi new                     Open $EDITOR and suggest a filename from its content
  ishi new meeting-notes       Create ./0-Inbox/meeting-notes.md directly
  ishi new --project apollo    Scaffold a new project directory")]
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
        /// Print the effective config as JSON instead of annotated TOML.
        #[arg(long)]
        json: bool,
    },
    /// List items in a category.
    #[command(alias = "ls")]
    List {
        category: ListCategory,
        filter: Option<String>,
        /// Print results as a JSON array instead of a table.
        #[arg(long)]
        json: bool,
    },
    /// Print a shell completion script for `ishi` to stdout.
    Completions { shell: CompletionShell },
    /// Print a per-category summary of the PARA system.
    Status {
        /// Print the report as a JSON object instead of text.
        #[arg(long)]
        json: bool,
    },
    /// Relocate an item to a different category.
    #[command(
        alias = "mv",
        after_help = "\
Examples:
  ishi move meeting-notes project   File an Inbox item as a project
  ishi mv apollo archive            Archive a project (prompts for a summary)
  ishi move apollo archive --yes    Archive it, accepting the suggested summary"
    )]
    Move {
        /// Name of the item to relocate, as shown by `ishi list`.
        #[arg(add = ArgValueCompleter::new(complete_move_name))]
        name: String,
        /// Category to move the item into.
        target: MoveTarget,
        /// Accept the suggested archive summary without prompting.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// File an item away — sugar for `ishi move <item> archive`.
    #[command(after_help = "\
Examples:
  ishi archive apollo         Archive \"apollo\" (prompts for a summary)
  ishi archive apollo --yes   Archive it, accepting the suggested summary")]
    Archive {
        /// Name of the item to archive, as shown by `ishi list`.
        #[arg(add = ArgValueCompleter::new(complete_archive_name))]
        name: String,
        /// Accept the suggested archive summary without prompting.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// Restore an archived item to the category it was archived from.
    #[command(after_help = "\
Examples:
  ishi unarchive Projects/apollo   Restore \"apollo\" to 1-Projects")]
    Unarchive {
        /// Qualified `<OriginCategory>/<name>` of the archived item, as
        /// shown by `ishi list archive`.
        #[arg(add = ArgValueCompleter::new(complete_unarchive_name))]
        name: String,
    },
    /// Walk every project and area, prompting keep/archive/skip.
    #[command(after_help = "\
Examples:
  ishi review                          Walk every project and area interactively
  ishi review website-redesign --keep  Stamp last_reviewed for one item, no prompt
  ishi review website-redesign         Prompt for just that one item")]
    Review {
        /// Name of the item to review, as shown by `ishi list`. Omit to
        /// walk every project and area.
        name: Option<String>,
        #[command(flatten)]
        decision: ReviewDecision,
    },
}

/// The `--keep`/`--archive`/`--skip` flags for `ishi review <name>` — at
/// most one may be given (`review.md` 004 scenario 6).
#[derive(Debug, Default, Clone, Copy, PartialEq, Args)]
#[group(multiple = false)]
struct ReviewDecision {
    /// Stamp `last_reviewed` to today and leave the item in place.
    #[arg(long)]
    keep: bool,
    /// Move the item to the Archive.
    #[arg(long)]
    archive: bool,
    /// Leave the item untouched.
    #[arg(long)]
    skip: bool,
}

impl ReviewDecision {
    fn into_decision(self) -> Option<review::Decision> {
        if self.keep {
            Some(review::Decision::Keep)
        } else if self.archive {
            Some(review::Decision::Archive)
        } else if self.skip {
            Some(review::Decision::Skip)
        } else {
            None
        }
    }
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
    let Ok(ws) = Workspace::discover(&cwd, home_ishi_toml().as_deref()) else {
        return vec![];
    };
    names(&ws)
        .into_iter()
        .filter(|name| cli::completion_candidate_matches(name, current))
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
/// glue that re-invokes `ishi` (via `$PATH`) at completion time — rather than
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
        .write_registration("COMPLETE", "ishi", "ishi", "ishi", &mut buf)
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
    /// Write a new `.ishi.toml` (or `~/.ishi.toml` with `-g`) populated
    /// with the built-in defaults.
    Init {
        #[arg(short = 'g', long = "global")]
        global: bool,
    },
    /// Open `.ishi.toml` (or `~/.ishi.toml` with `-g`) in `$EDITOR`,
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

/// Resolves `~/.ishi.toml`, or `None` if `$HOME` isn't set.
fn home_ishi_toml() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".ishi.toml"))
}

/// Computes the local-vs-global config target: the path to write/open, and
/// its human-readable display form (`"./.ishi.toml"` or `"~/.ishi.toml"`).
fn config_target(cwd: &Path, global: bool) -> anyhow::Result<(PathBuf, String)> {
    Ok(if global {
        let path = home_ishi_toml().context("$HOME is not set")?;
        (path, "~/.ishi.toml".to_string())
    } else {
        (cwd.join(".ishi.toml"), "./.ishi.toml".to_string())
    })
}

fn run_daily_command(ws: &Workspace) -> anyhow::Result<()> {
    if cli::daily_note_exists(ws) {
        println!("Opening $EDITOR...");
    }
    let editor = RealEditor;
    if let cli::DailyOutcome::Created(path) = cli::run_daily(ws, &editor)? {
        println!("Created {}", cli::display_path(ws, &path));
        println!("Next: ishi list to see it, or ishi status for an overview.");
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    clap_complete::CompleteEnv::with_factory(Cli::command).complete();

    env_logger::init();
    let cli = Cli::parse();

    let cwd = env::current_dir().context("failed to determine current directory")?;
    let home_config = home_ishi_toml();

    match cli.command {
        Commands::Init { name } => {
            let message = cli::run_init(&cwd, name.as_deref(), home_config.as_deref())?;
            println!("{message}");
            match name.as_deref() {
                Some(n) => println!("Next: cd {n} && ishi new to capture your first note."),
                None => println!("Next: ishi new to capture your first note."),
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
            println!("Created {}", cli::display_path(&ws, &path));
            println!("Next: ishi list to see it, or ishi status for an overview.");
        }
        Commands::Config {
            action: Some(_),
            json: true,
        } => {
            anyhow::bail!("--json cannot be combined with a config subcommand");
        }
        Commands::Config {
            action: Some(ConfigAction::Init { global }),
            json: false,
        } => {
            let (path, display) = config_target(&cwd, global)?;
            let message = cli::run_config_init(&path, &display)?;
            println!("{message}");
        }
        Commands::Config {
            action: Some(ConfigAction::Edit { global }),
            json: false,
        } => {
            let (path, display) = config_target(&cwd, global)?;
            if !path.exists() {
                println!("Created {display}");
            }
            println!("Opening $EDITOR...");
            let editor = RealEditor;
            cli::run_config_edit(&path, &editor)?;
        }
        Commands::Config { action: None, json } => {
            let (path, _display) = config_target(&cwd, false)?;
            let (config, origins) = ishi::config::Config::resolve(&path, home_config.as_deref())?;
            if json {
                print!("{}", ishi::config::render_effective_json(&config, &origins));
            } else {
                print!("{}", ishi::config::render_effective(&config, &origins));
            }
        }
        Commands::List {
            category,
            filter,
            json,
        } => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            let output = if json {
                cli::run_list_json(&ws, category.into(), filter.as_deref())?
            } else {
                cli::run_list(&ws, category.into(), filter.as_deref())?
            };
            println!("{output}");
        }
        Commands::Completions { shell } => {
            io::stdout().write_all(&render_completions(shell))?;
        }
        Commands::Status { json } => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            let output = if json {
                cli::run_status_json(&ws)?
            } else {
                cli::run_status(&ws)?
            };
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
        Commands::Review {
            name: Some(name),
            decision,
        } => {
            let ws = Workspace::discover(&cwd, home_config.as_deref())
                .context("failed to find a PARA workspace")?;
            let mut ui = TerminalUi;
            if let Some(message) = review::run_one(&ws, &mut ui, &name, decision.into_decision())? {
                println!("{message}");
            }
        }
        Commands::Review {
            name: None,
            decision,
        } if decision.into_decision().is_some() => {
            anyhow::bail!("--keep/--archive/--skip requires an item name");
        }
        Commands::Review { name: None, .. } => {
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
        let cli = Cli::parse_from(["ishi", "new", "my-file"]);

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
        let cli = Cli::parse_from(["ishi", "new", "--project", "website-redesign"]);

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
        let cli = Cli::parse_from(["ishi", "new", "--area", "health"]);

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
        let cli = Cli::parse_from(["ishi", "new", "--resource", "recipe-ideas"]);

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
        let cli = Cli::parse_from(["ishi", "new", "--yes"]);

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
        let cli = Cli::parse_from(["ishi", "new", "-y"]);

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
        let result = Cli::try_parse_from(["ishi", "new", "--project", "--area", "x"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_new_daily() {
        let cli = Cli::parse_from(["ishi", "new", "--daily"]);

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
        let result = Cli::try_parse_from(["ishi", "new", "--daily", "x"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_new_daily_with_project() {
        let result = Cli::try_parse_from(["ishi", "new", "--daily", "--project"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_daily() {
        let cli = Cli::parse_from(["ishi", "daily"]);

        assert_eq!(cli.command, Commands::Daily);
    }

    #[test]
    fn parses_status() {
        let cli = Cli::parse_from(["ishi", "status"]);

        assert_eq!(cli.command, Commands::Status { json: false });
    }

    #[test]
    fn parses_move() {
        let cli = Cli::parse_from(["ishi", "move", "my-file", "project"]);

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
        let cli = Cli::parse_from(["ishi", "mv", "my-file", "archive"]);

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
        let cli = Cli::parse_from(["ishi", "archive", "my-file"]);

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
        let cli = Cli::parse_from(["ishi", "archive", "my-file", "--yes"]);

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
        let cli = Cli::parse_from(["ishi", "move", "my-file", "archive", "-y"]);

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
        let result = Cli::try_parse_from(["ishi", "archive", "my-file", "archive"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_unarchive() {
        let cli = Cli::parse_from(["ishi", "unarchive", "Projects/my-file"]);

        assert_eq!(
            cli.command,
            Commands::Unarchive {
                name: "Projects/my-file".to_string(),
            }
        );
    }

    #[test]
    fn rejects_unarchive_with_category_argument() {
        let result = Cli::try_parse_from(["ishi", "unarchive", "Projects/my-file", "project"]);

        assert!(result.is_err());
    }

    #[test]
    fn parses_review() {
        let cli = Cli::parse_from(["ishi", "review"]);

        assert_eq!(
            cli.command,
            Commands::Review {
                name: None,
                decision: ReviewDecision::default(),
            }
        );
    }

    #[test]
    fn parses_review_with_name_and_no_flags() {
        let cli = Cli::parse_from(["ishi", "review", "website-redesign"]);

        assert_eq!(
            cli.command,
            Commands::Review {
                name: Some("website-redesign".to_string()),
                decision: ReviewDecision::default(),
            }
        );
    }

    #[test]
    fn parses_review_with_archive_flag() {
        let cli = Cli::parse_from(["ishi", "review", "website-redesign", "--archive"]);

        assert_eq!(
            cli.command,
            Commands::Review {
                name: Some("website-redesign".to_string()),
                decision: ReviewDecision {
                    archive: true,
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn rejects_conflicting_review_decision_flags() {
        let result =
            Cli::try_parse_from(["ishi", "review", "website-redesign", "--keep", "--archive"]);

        assert!(result.is_err());
    }

    #[test]
    fn review_keep_with_no_name_parses_but_is_flagged_for_dispatch_rejection() {
        let cli = Cli::parse_from(["ishi", "review", "--keep"]);

        let Commands::Review { name, decision } = cli.command else {
            panic!("expected Commands::Review");
        };
        assert_eq!(name, None);
        assert!(
            decision.into_decision().is_some(),
            "dispatch in main() bails on (None, decision) when a decision flag is set"
        );
    }

    #[test]
    fn rejects_daily_with_filename() {
        let result = Cli::try_parse_from(["ishi", "daily", "x"]);

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
        let cli = Cli::parse_from(["ishi", "init", "my-para"]);

        assert_eq!(
            cli.command,
            Commands::Init {
                name: Some("my-para".to_string())
            }
        );
    }

    #[test]
    fn parses_init_without_name() {
        let cli = Cli::parse_from(["ishi", "init"]);

        assert_eq!(cli.command, Commands::Init { name: None });
    }

    #[test]
    fn parses_config_init_with_no_flag() {
        let cli = Cli::parse_from(["ishi", "config", "init"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Init { global: false }),
                json: false,
            }
        );
    }

    #[test]
    fn parses_config_init_global_short_flag() {
        let cli = Cli::parse_from(["ishi", "config", "init", "-g"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Init { global: true }),
                json: false,
            }
        );
    }

    #[test]
    fn parses_config_init_global_long_flag() {
        let cli = Cli::parse_from(["ishi", "config", "init", "--global"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Init { global: true }),
                json: false,
            }
        );
    }

    #[test]
    fn parses_config_edit_with_no_flag() {
        let cli = Cli::parse_from(["ishi", "config", "edit"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Edit { global: false }),
                json: false,
            }
        );
    }

    #[test]
    fn parses_config_edit_global_short_flag() {
        let cli = Cli::parse_from(["ishi", "config", "edit", "-g"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Edit { global: true }),
                json: false,
            }
        );
    }

    #[test]
    fn parses_config_edit_global_long_flag() {
        let cli = Cli::parse_from(["ishi", "config", "edit", "--global"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: Some(ConfigAction::Edit { global: true }),
                json: false,
            }
        );
    }

    #[test]
    fn parses_config_bare_as_action_none() {
        let cli = Cli::parse_from(["ishi", "config"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: None,
                json: false,
            }
        );
    }

    #[test]
    fn parses_config_json_flag() {
        let cli = Cli::parse_from(["ishi", "config", "--json"]);

        assert_eq!(
            cli.command,
            Commands::Config {
                action: None,
                json: true,
            }
        );
    }

    #[test]
    fn parses_list_project() {
        let cli = Cli::parse_from(["ishi", "list", "project"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Project,
                filter: None,
                json: false,
            }
        );
    }

    #[test]
    fn parses_list_project_with_filter() {
        let cli = Cli::parse_from(["ishi", "list", "project", "web"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Project,
                filter: Some("web".into()),
                json: false,
            }
        );
    }

    #[test]
    fn parses_list_area() {
        let cli = Cli::parse_from(["ishi", "list", "area"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Area,
                filter: None,
                json: false,
            }
        );
    }

    #[test]
    fn parses_list_resource() {
        let cli = Cli::parse_from(["ishi", "list", "resource"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Resource,
                filter: None,
                json: false,
            }
        );
    }

    #[test]
    fn parses_list_inbox() {
        let cli = Cli::parse_from(["ishi", "list", "inbox"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Inbox,
                filter: None,
                json: false,
            }
        );
    }

    #[test]
    fn parses_list_archive() {
        let cli = Cli::parse_from(["ishi", "list", "archive"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Archive,
                filter: None,
                json: false,
            }
        );
    }

    #[test]
    fn parses_list_json_flag() {
        let cli = Cli::parse_from(["ishi", "list", "project", "--json"]);

        assert_eq!(
            cli.command,
            Commands::List {
                category: ListCategory::Project,
                filter: None,
                json: true,
            }
        );
    }

    #[test]
    fn parses_status_json_flag() {
        let cli = Cli::parse_from(["ishi", "status", "--json"]);

        assert_eq!(cli.command, Commands::Status { json: true });
    }

    #[test]
    fn parses_completions_bash() {
        let cli = Cli::parse_from(["ishi", "completions", "bash"]);

        assert_eq!(
            cli.command,
            Commands::Completions {
                shell: CompletionShell::Bash
            }
        );
    }

    #[test]
    fn parses_completions_zsh() {
        let cli = Cli::parse_from(["ishi", "completions", "zsh"]);

        assert_eq!(
            cli.command,
            Commands::Completions {
                shell: CompletionShell::Zsh
            }
        );
    }

    #[test]
    fn parses_completions_fish() {
        let cli = Cli::parse_from(["ishi", "completions", "fish"]);

        assert_eq!(
            cli.command,
            Commands::Completions {
                shell: CompletionShell::Fish
            }
        );
    }

    #[test]
    fn parses_completions_powershell() {
        let cli = Cli::parse_from(["ishi", "completions", "powershell"]);

        assert_eq!(
            cli.command,
            Commands::Completions {
                shell: CompletionShell::Powershell
            }
        );
    }

    #[test]
    fn rejects_unsupported_completions_shell() {
        let result = Cli::try_parse_from(["ishi", "completions", "tcsh"]);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_missing_completions_shell() {
        let result = Cli::try_parse_from(["ishi", "completions"]);

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
        let candidates = complete(&["ishi", ""], 1);

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

    /// Writes an empty `.ishi.toml` marker at `root` (matching
    /// `workspace::init`'s discovery contract) and returns a `Workspace`
    /// rooted there with default config, for tests that need a real
    /// on-disk PARA system for `Workspace::discover` to find via cwd.
    fn init_workspace(root: &Path) -> Workspace {
        fs::write(root.join(".ishi.toml"), "").unwrap();
        Workspace {
            root: root.to_path_buf(),
            config: ishi::config::Config::default(),
        }
    }

    #[test]
    fn completes_a_live_items_bare_name() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        ishi::items::create(&ws, ishi::category::Category::Inbox, "my-file", "hello").unwrap();
        ishi::items::create(
            &ws,
            ishi::category::Category::Project,
            "website-redesign",
            "",
        )
        .unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["ishi", "move", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        let mut candidates = candidates;
        candidates.sort();
        assert_eq!(candidates, vec!["my-file", "website-redesign"]);
    }

    #[test]
    fn completes_qualified_forms_for_colliding_live_item_names() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        ishi::items::create(&ws, ishi::category::Category::Inbox, "meeting-notes", "").unwrap();
        ishi::items::create(&ws, ishi::category::Category::Resource, "meeting-notes", "").unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["ishi", "move", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        let mut candidates = candidates;
        candidates.sort();
        assert_eq!(
            candidates,
            vec!["inbox/meeting-notes", "resources/meeting-notes"]
        );
    }

    #[test]
    fn completes_bare_prefix_of_colliding_live_item_names() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        ishi::items::create(&ws, ishi::category::Category::Inbox, "meeting-notes", "").unwrap();
        ishi::items::create(&ws, ishi::category::Category::Resource, "meeting-notes", "").unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["ishi", "move", "meeti"], 2);

        env::set_current_dir(original_cwd).unwrap();
        let mut candidates = candidates;
        candidates.sort();
        assert_eq!(
            candidates,
            vec!["inbox/meeting-notes", "resources/meeting-notes"]
        );
    }

    #[test]
    fn completes_qualified_prefix_scopes_to_one_category() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        ishi::items::create(&ws, ishi::category::Category::Inbox, "meeting-notes", "").unwrap();
        ishi::items::create(&ws, ishi::category::Category::Resource, "meeting-notes", "").unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["ishi", "move", "inbox/meeti"], 2);

        env::set_current_dir(original_cwd).unwrap();
        assert_eq!(candidates, vec!["inbox/meeting-notes"]);
    }

    #[test]
    fn completes_an_archived_items_qualified_name() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        let path =
            ishi::items::create(&ws, ishi::category::Category::Inbox, "meeting-notes", "").unwrap();
        ishi::items::mv(
            &ws,
            ishi::category::Category::Inbox,
            &path,
            "meeting-notes",
            ishi::category::Category::Archive,
        )
        .unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["ishi", "unarchive", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        assert_eq!(candidates, vec!["Inbox/meeting-notes"]);
    }

    #[test]
    fn archive_completion_excludes_archived_items() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        ishi::items::create(&ws, ishi::category::Category::Project, "apollo", "").unwrap();
        let path =
            ishi::items::create(&ws, ishi::category::Category::Inbox, "meeting-notes", "").unwrap();
        ishi::items::mv(
            &ws,
            ishi::category::Category::Inbox,
            &path,
            "meeting-notes",
            ishi::category::Category::Archive,
        )
        .unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let candidates = complete(&["ishi", "archive", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        assert_eq!(candidates, vec!["apollo"]);
    }

    #[test]
    fn move_completion_includes_both_live_and_archived_items() {
        let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_cwd = env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let ws = init_workspace(dir.path());
        ishi::items::create(&ws, ishi::category::Category::Project, "apollo", "").unwrap();
        let path =
            ishi::items::create(&ws, ishi::category::Category::Inbox, "meeting-notes", "").unwrap();
        ishi::items::mv(
            &ws,
            ishi::category::Category::Inbox,
            &path,
            "meeting-notes",
            ishi::category::Category::Archive,
        )
        .unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let mut candidates = complete(&["ishi", "move", ""], 2);

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
        ishi::items::create(&first_ws, ishi::category::Category::Inbox, "first-item", "").unwrap();
        let second = tempfile::tempdir().unwrap();
        let second_ws = init_workspace(second.path());
        ishi::items::create(
            &second_ws,
            ishi::category::Category::Inbox,
            "second-item",
            "",
        )
        .unwrap();

        env::set_current_dir(first.path()).unwrap();
        let first_candidates = complete(&["ishi", "move", ""], 2);
        env::set_current_dir(second.path()).unwrap();
        let second_candidates = complete(&["ishi", "move", ""], 2);

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

        let candidates = complete(&["ishi", "move", ""], 2);

        env::set_current_dir(original_cwd).unwrap();
        assert!(candidates.is_empty());
    }
}

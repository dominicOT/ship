use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
// use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

mod checks;
mod config;
mod export;
mod init;
mod project;
mod update;
mod util;

use checks::{CheckResult, CheckStatus};
use project::Project;

#[derive(Parser, Debug)]
#[command(
    name = "ship",
    version,
    about = "One command pre-deploy checklist",
    long_about = "Run essential checks before deploying:\n  ✓ tests\n  ✓ secrets\n  ✓ TODOs\n  ✓ console.logs\n  ✓ feature flags\n  ✓ version\n  ✓ migrations\n  ✓ changelog"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to project directory (defaults to current directory)
    #[arg(long, short = 'p', value_name = "DIR", global = true)]
    project: Option<PathBuf>,

    /// Run checks without failing (report only)
    #[arg(long, short = 'n')]
    dry_run: bool,

    /// Skip specific checks (comma-separated: tests,secrets,todos,logs,flags,version,migrations,changelog)
    #[arg(long, value_delimiter = ',')]
    skip: Vec<String>,

    /// Verbose output
    #[arg(long, short)]
    verbose: bool,

    /// Only run these checks (comma-separated)
    #[arg(long, value_delimiter = ',')]
    only: Vec<String>,

    /// Export report as JSON to optional path (defaults to `ship-report.json` when flag provided without a path)
    #[arg(long, value_name = "FILE", num_args = 0..=1, default_missing_value = "ship-report.json")]
    json: Option<PathBuf>,

    /// Export report as Markdown to optional path (defaults to `ship-report.md` when flag provided without a path)
    #[arg(long, value_name = "FILE", num_args = 0..=1, default_missing_value = "ship-report.md")]
    md: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install ship as a committed git pre-commit hook
    Init {
        /// Custom hook directory (defaults to .githooks)
        #[arg(long, default_value = ".githooks")]
        hook_dir: String,

        /// Overwrite existing pre-commit hook if present
        #[arg(long, short = 'f')]
        force: bool,

        /// Do not create .ship.toml config file
        #[arg(long)]
        no_config: bool,

        /// Checks the pre-commit hook should skip (comma-separated). Defaults to "tests".
        #[arg(long, value_delimiter = ',')]
        skip: Vec<String>,

        /// Checks the pre-commit hook should exclusively run (comma-separated)
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,

        /// Run every check in the pre-commit hook (overrides the default tests skip)
        #[arg(long)]
        all: bool,
    },

    /// Update ship to the latest GitHub release
    Update {
        /// Only check whether an update is available, don't install it
        #[arg(long)]
        check: bool,

        /// Reinstall the latest release even if already up to date
        #[arg(long, short = 'f')]
        force: bool,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(success) => {
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("{} {:#}", "error:".red().bold(), e);
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool> {
    let cli = Cli::parse();

    if let Some(Commands::Update { check, force }) = cli.command {
        update::run(&update::UpdateOptions { check, force })?;
        return Ok(true);
    }

    let project = Project::detect_from_path(cli.project.as_deref())
        .context("Failed to detect project")?;

    if let Some(Commands::Init {
        ref hook_dir,
        force,
        no_config,
        ref skip,
        ref only,
        all,
    }) = cli.command
    {
        init::run(
            project.root_path(),
            &init::InitOptions {
                hook_dir,
                force,
                no_config,
                skip,
                only,
                all,
            },
        )?;
        return Ok(true);
    }

    println!("{}", "ship".bold().cyan());
    println!();
    println!("{}", "Checks".bold());

    if cli.verbose {
        println!("  Project type: {}", project.kind);
        if let Some(ref root) = project.root {
            println!("  Root: {}", root.display());
        }
        println!();
    }

    let config = config::ShipConfig::load_from_dir(project.root_path()).unwrap_or_default();

    let effective_skip = if !cli.skip.is_empty() {
        cli.skip
    } else {
        config.skip
    };

    let effective_only = if !cli.only.is_empty() {
        cli.only
    } else {
        config.only
    };

    let mut results: Vec<CheckResult> = Vec::new();

    let to_run: Vec<&str> = if !effective_only.is_empty() {
        checks::ALL_CHECKS
            .iter()
            .filter(|c| effective_only.iter().any(|o| o.eq_ignore_ascii_case(c)))
            .copied()
            .collect()
    } else {
        checks::ALL_CHECKS
            .iter()
            .filter(|c| !effective_skip.iter().any(|s| s.eq_ignore_ascii_case(c)))
            .copied()
            .collect()
    };

    for name in to_run {
        let result = match name {
            "tests" => checks::tests::run(&project, cli.verbose),
            "secrets" => checks::secrets::run(&project, cli.verbose),
            "todos" => checks::todos::run(&project, cli.verbose),
            "logs" => checks::logs::run(&project, cli.verbose),
            "flags" => checks::flags::run(&project, cli.verbose),
            "version" => checks::version::run(&project, cli.verbose),
            "migrations" => checks::migrations::run(&project, cli.verbose),
            "changelog" => checks::changelog::run(&project, cli.verbose),
            _ => unreachable!(),
        };

        print_result(&result);
        results.push(result);
    }

    println!();

    if cli.json.is_some() || cli.md.is_some() {
        if let Some(path) = cli.json.as_ref() {
            export::write_json(path.as_path(), &project, &results)
                .context("Failed to write JSON report")?;
            println!("Wrote JSON report to {}", path.display());
        }

        if let Some(path) = cli.md.as_ref() {
            export::write_markdown(path.as_path(), &project, &results)
                .context("Failed to write Markdown report")?;
            println!("Wrote Markdown report to {}", path.display());
        }
    }

    let critical_failed = results
        .iter()
        .any(|r| matches!(r.status, CheckStatus::Fail) && r.critical);

    let any_failed = results
        .iter()
        .any(|r| matches!(r.status, CheckStatus::Fail));

    if critical_failed && !cli.dry_run {
        println!("{}", "✗ Not ready to ship".red().bold());
        Ok(false)
    } else if any_failed {
        println!("{}", "⚠ Ready with warnings".yellow().bold());
        Ok(true)
    } else {
        println!("{}", "✓ Ready to ship".green().bold());
        Ok(true)
    }
}

fn print_result(result: &CheckResult) {
    let icon = match result.status {
        CheckStatus::Pass => "✓".green(),
        CheckStatus::Fail => "✗".red(),
        CheckStatus::Skip => "–".dimmed(),
        CheckStatus::Warn => "!".yellow(),
    };

    let name = match result.status {
        CheckStatus::Pass => result.name.green(),
        CheckStatus::Fail => result.name.red(),
        CheckStatus::Skip => result.name.dimmed(),
        CheckStatus::Warn => result.name.yellow(),
    };

    print!("  {} {}", icon, name);

    if let Some(ref detail) = result.detail {
        print!("  {}", detail.dimmed());
    }

    println!();

    if let Some(ref extra) = result.extra {
        for line in extra.lines() {
            println!("      {}", line.dimmed());
        }
    }
}

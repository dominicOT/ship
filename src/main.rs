use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use std::process::ExitCode;

mod checks;
mod util;
mod project;

use checks::{CheckResult, CheckStatus};
use project::Project;

#[derive(Parser, Debug)]
#[command(
    name = "ship",
    about = "One command pre-deploy checklist",
    long_about = "Run essential checks before deploying:\n  ✓ tests\n  ✓ secrets\n  ✓ TODOs\n  ✓ console.logs\n  ✓ feature flags\n  ✓ version\n  ✓ migrations\n  ✓ changelog"
)]
struct Cli {
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
            eprintln!("{} {}", "error:".red().bold(), e);
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool> {
    let cli = Cli::parse();

    println!("{}", "ship".bold().cyan());
    println!();
    println!("{}", "Checks".bold());

    let project = Project::detect().context("Failed to detect project")?;

    if cli.verbose {
        println!("  Project type: {}", project.kind);
        if let Some(ref root) = project.root {
            println!("  Root: {}", root.display());
        }
        println!();
    }

    let mut results: Vec<CheckResult> = Vec::new();

    let all_checks = [
        "tests",
        "secrets",
        "todos",
        "logs",
        "flags",
        "version",
        "migrations",
        "changelog",
    ];

    let to_run: Vec<&str> = if !cli.only.is_empty() {
        all_checks
            .iter()
            .filter(|c| cli.only.iter().any(|o| o.eq_ignore_ascii_case(c)))
            .copied()
            .collect()
    } else {
        all_checks
            .iter()
            .filter(|c| !cli.skip.iter().any(|s| s.eq_ignore_ascii_case(c)))
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

    let critical_failed = results
        .iter()
        .any(|r| matches!(r.status, CheckStatus::Fail) && r.critical);

    let any_failed = results.iter().any(|r| matches!(r.status, CheckStatus::Fail));

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

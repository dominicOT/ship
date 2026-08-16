use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
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

    /// Export report as JSON to optional path (defaults to `ship-report.json` when flag provided without a path)
    #[arg(long, value_name = "FILE", num_args = 0..=1, default_missing_value = "ship-report.json")]
    json: Option<PathBuf>,

    /// Export report as Markdown to optional path (defaults to `ship-report.md` when flag provided without a path)
    #[arg(long, value_name = "FILE", num_args = 0..=1, default_missing_value = "ship-report.md")]
    md: Option<PathBuf>,
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

    if cli.json.is_some() || cli.md.is_some() {
        #[derive(Serialize)]
        struct ExportProject {
            kind: String,
            root: Option<String>,
        }

        #[derive(Serialize)]
        struct ExportReport<'a> {
            project: ExportProject,
            results: &'a Vec<CheckResult>,
        }

        let exp_project = ExportProject {
            kind: project.kind.to_string(),
            root: project.root.as_ref().map(|p| p.display().to_string()),
        };

        let report = ExportReport {
            project: exp_project,
            results: &results,
        };

        if let Some(path) = cli.json.as_ref() {
            let s = serde_json::to_string_pretty(&report).context("Failed to serialize JSON report")?;
            fs::write(path, s).context("Failed to write JSON report")?;
            println!("Wrote JSON report to {}", path.display());
        }

        if let Some(path) = cli.md.as_ref() {
            let mut md = String::new();
            md.push_str("# ship report\n\n");
            md.push_str(&format!("**Project:** {}\n\n", project.kind));
            if let Some(ref root) = project.root {
                md.push_str(&format!("**Root:** {}\n\n", root.display()));
            }
            md.push_str("## Checks\n\n");

            for r in &results {
                let status = match &r.status {
                    CheckStatus::Pass => "pass",
                    CheckStatus::Fail => "fail",
                    CheckStatus::Warn => "warn",
                    CheckStatus::Skip => "skip",
                };
                md.push_str(&format!("- **{}** — {}\n", r.name, status));
                if let Some(ref d) = r.detail {
                    md.push_str(&format!("  - {}\n", d));
                }
                if let Some(ref e) = r.extra {
                    md.push_str("\n```");
                    md.push_str(e);
                    md.push_str("```\n\n");
                }
            }

            fs::write(path, md).context("Failed to write Markdown report")?;
            println!("Wrote Markdown report to {}", path.display());
        }
    }

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

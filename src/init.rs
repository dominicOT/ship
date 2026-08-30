use anyhow::{bail, Context, Result};
use colored::*;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::checks::ALL_CHECKS;

pub struct InitOptions<'a> {
    pub hook_dir: &'a str,
    #[allow(dead_code)]
    pub force: bool,
    pub no_config: bool,
    pub skip: &'a [String],
    pub only: &'a [String],
    pub all: bool,
}

pub fn run(root: &Path, options: &InitOptions) -> Result<()> {
    println!("{}", "ship init".bold().cyan());
    println!();

    validate_checks("--skip", options.skip)?;
    validate_checks("--only", options.only)?;

    // 1. Create hooks directory (e.g. .githooks)
    let hooks_dir = root.join(options.hook_dir);
    fs::create_dir_all(&hooks_dir)
        .with_context(|| format!("Failed to create directory: {}", hooks_dir.display()))?;

    // 2. Write pre-commit hook
    let hook_path = hooks_dir.join("pre-commit");
    let hook_exists = hook_path.exists();

    let ship_invocation = if !options.only.is_empty() {
        format!("ship --only {}", options.only.join(","))
    } else if !options.skip.is_empty() {
        format!("ship --skip {}", options.skip.join(","))
    } else if options.all {
        "ship".to_string()
    } else {
        // Default: skip the (usually slow) test suite in the hook.
        "ship --skip tests".to_string()
    };

    let hook_content = format!(
        "#!/bin/sh\n# .githooks/pre-commit — managed by ship\nif ! command -v ship >/dev/null 2>&1; then\n  echo \"ship is not installed. See https://github.com/dominicOT/ship\"\n  exit 1\nfi\nexec {ship_invocation}\n"
    );

    fs::write(&hook_path, &hook_content)
        .with_context(|| format!("Failed to write hook file: {}", hook_path.display()))?;

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = fs::metadata(&hook_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&hook_path, perms);
        }
    }

    let hook_display = format!("{}/pre-commit", options.hook_dir);
    if hook_exists {
        println!("  {} Updated {}", "✓".green(), hook_display);
    } else {
        println!("  {} Created {}", "✓".green(), hook_display);
    }
    println!("      runs: {}", ship_invocation.dimmed());

    // 3. Configure git core.hooksPath
    let is_git_repo = is_inside_git_repo(root);
    if is_git_repo {
        let status = Command::new("git")
            .args(["config", "core.hooksPath", options.hook_dir])
            .current_dir(root)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!(
                    "  {} Configured git hooks: {}",
                    "✓".green(),
                    format!("git config core.hooksPath {}", options.hook_dir).bold()
                );
            }
            Ok(_) => {
                println!(
                    "  {} Failed to set git config core.hooksPath",
                    "⚠".yellow()
                );
            }
            Err(e) => {
                println!("  {} Could not run git: {}", "⚠".yellow(), e);
            }
        }
    } else {
        println!(
            "  {} Not a git repository (run {} after git init)",
            "⚠".yellow(),
            format!("git config core.hooksPath {}", options.hook_dir).bold()
        );
    }

    // 4. Optionally write .ship.toml
    if !options.no_config {
        let config_path = root.join(".ship.toml");
        if !config_path.exists() {
            let default_config = "# .ship.toml — Configuration for ship\n# Documentation: https://github.com/dominicOT/ship\n\n# Checks to skip by default (e.g. tests, secrets, todos, logs, flags, version, migrations, changelog)\n# skip = [\"tests\"]\n\n# Only run specific checks\n# only = [\"secrets\", \"todos\"]\n";
            if fs::write(&config_path, default_config).is_ok() {
                println!("  {} Created .ship.toml", "✓".green());
            }
        } else {
            println!("  – .ship.toml already exists (kept)");
        }
    }

    println!();
    println!("{}", "Next steps:".bold());
    println!("  git add {} .ship.toml", options.hook_dir);
    println!("  git commit -m \"chore: add ship pre-commit hook\"");
    println!();
    println!("Teammates just need to run once:");
    println!("  git config core.hooksPath {}", options.hook_dir);
    println!();

    Ok(())
}

fn validate_checks(flag: &str, checks: &[String]) -> Result<()> {
    for check in checks {
        if !ALL_CHECKS.iter().any(|c| c.eq_ignore_ascii_case(check)) {
            bail!(
                "Unknown check '{check}' passed to {flag}. Valid checks: {}",
                ALL_CHECKS.join(", ")
            );
        }
    }
    Ok(())
}

fn is_inside_git_repo(dir: &Path) -> bool {
    if dir.join(".git").exists() {
        return true;
    }
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_hook_and_config() {
        let temp_dir = std::env::temp_dir().join(format!("ship_test_init_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let options = InitOptions {
            hook_dir: ".githooks",
            force: false,
            no_config: false,
            skip: &[],
            only: &[],
            all: false,
        };

        let res = run(&temp_dir, &options);
        assert!(res.is_ok());

        let hook_file = temp_dir.join(".githooks/pre-commit");
        assert!(hook_file.is_file());
        let content = fs::read_to_string(&hook_file).expect("read hook");
        assert!(content.contains("#!/bin/sh"));
        assert!(content.contains("exec ship --skip tests"));

        let config_file = temp_dir.join(".ship.toml");
        assert!(config_file.is_file());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn init_no_config_flag_skips_ship_toml() {
        let temp_dir = std::env::temp_dir().join(format!("ship_test_init_noconf_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let options = InitOptions {
            hook_dir: ".githooks",
            force: false,
            no_config: true,
            skip: &[],
            only: &[],
            all: false,
        };

        let res = run(&temp_dir, &options);
        assert!(res.is_ok());

        let hook_file = temp_dir.join(".githooks/pre-commit");
        assert!(hook_file.is_file());

        let config_file = temp_dir.join(".ship.toml");
        assert!(!config_file.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn init_skip_flag_overrides_default_hook() {
        let temp_dir =
            std::env::temp_dir().join(format!("ship_test_init_skip_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let skip = vec!["tests".to_string(), "logs".to_string()];
        let options = InitOptions {
            hook_dir: ".githooks",
            force: false,
            no_config: true,
            skip: &skip,
            only: &[],
            all: false,
        };

        let res = run(&temp_dir, &options);
        assert!(res.is_ok());

        let content = fs::read_to_string(temp_dir.join(".githooks/pre-commit")).expect("read hook");
        assert!(content.contains("exec ship --skip tests,logs"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn init_only_flag_takes_precedence_over_skip() {
        let temp_dir =
            std::env::temp_dir().join(format!("ship_test_init_only_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let skip = vec!["tests".to_string()];
        let only = vec!["secrets".to_string(), "todos".to_string()];
        let options = InitOptions {
            hook_dir: ".githooks",
            force: false,
            no_config: true,
            skip: &skip,
            only: &only,
            all: false,
        };

        let res = run(&temp_dir, &options);
        assert!(res.is_ok());

        let content = fs::read_to_string(temp_dir.join(".githooks/pre-commit")).expect("read hook");
        assert!(content.contains("exec ship --only secrets,todos"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn init_all_flag_runs_every_check() {
        let temp_dir =
            std::env::temp_dir().join(format!("ship_test_init_all_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let options = InitOptions {
            hook_dir: ".githooks",
            force: false,
            no_config: true,
            skip: &[],
            only: &[],
            all: true,
        };

        let res = run(&temp_dir, &options);
        assert!(res.is_ok());

        let content = fs::read_to_string(temp_dir.join(".githooks/pre-commit")).expect("read hook");
        assert!(content.contains("exec ship\n"));
        assert!(!content.contains("--skip"));
        assert!(!content.contains("--only"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn init_rejects_unknown_check_name() {
        let temp_dir =
            std::env::temp_dir().join(format!("ship_test_init_bad_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let skip = vec!["nope".to_string()];
        let options = InitOptions {
            hook_dir: ".githooks",
            force: false,
            no_config: true,
            skip: &skip,
            only: &[],
            all: false,
        };

        let res = run(&temp_dir, &options);
        assert!(res.is_err());
        assert!(!temp_dir.join(".githooks/pre-commit").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

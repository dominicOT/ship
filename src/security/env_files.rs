use crate::checks::CheckResult;
use crate::project::Project;
use crate::util::command_exists;
use std::process::Command;

/// `.env*` filenames that are meant to be committed — templates and
/// examples, not real secrets — so they're excluded from the flag.
const SAFE_SUFFIXES: &[&str] = &[
    ".example",
    ".sample",
    ".template",
    ".dist",
    ".defaults",
    ".test",
];

pub fn run(project: &Project, _verbose: bool) -> CheckResult {
    let root = project.root_path();

    if !root.join(".git").exists() {
        return CheckResult::skip("env-files", "not a git repository");
    }

    if !command_exists("git") {
        return CheckResult::skip("env-files", "git not found");
    }

    let output = match Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return CheckResult::skip("env-files", "failed to list git-tracked files"),
    };

    let tracked: Vec<String> = output
        .stdout
        .split(|&b| b == 0)
        .filter_map(|chunk| {
            if chunk.is_empty() {
                None
            } else {
                Some(String::from_utf8_lossy(chunk).into_owned())
            }
        })
        .collect();

    let mut flagged: Vec<String> = tracked
        .into_iter()
        .filter(|path| is_flagged_env_file(path))
        .collect();
    flagged.sort();

    if flagged.is_empty() {
        CheckResult::pass_with("env-files", "no .env files tracked by git")
    } else {
        let count = flagged.len();
        let detail = format!(
            "{} .env file{} tracked by git",
            count,
            if count == 1 { "" } else { "s" }
        );
        CheckResult::fail("env-files", detail).with_extra(flagged.join("\n"))
    }
}

fn is_flagged_env_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);

    if name != ".env" && !name.starts_with(".env.") {
        return false;
    }

    let lower = name.to_ascii_lowercase();
    !SAFE_SUFFIXES.iter().any(|suffix| lower.ends_with(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_real_env_files() {
        assert!(is_flagged_env_file(".env"));
        assert!(is_flagged_env_file(".env.local"));
        assert!(is_flagged_env_file(".env.production"));
        assert!(is_flagged_env_file("config/.env"));
    }

    #[test]
    fn does_not_flag_templates_or_unrelated_files() {
        assert!(!is_flagged_env_file(".env.example"));
        assert!(!is_flagged_env_file(".env.sample"));
        assert!(!is_flagged_env_file(".env.template"));
        assert!(!is_flagged_env_file(".environment"));
        assert!(!is_flagged_env_file("env.rs"));
        assert!(!is_flagged_env_file("Cargo.toml"));
    }
}

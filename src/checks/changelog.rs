use crate::checks::CheckResult;
use crate::project::Project;
use std::fs;
use std::process::Command;

pub fn run(project: &Project, _verbose: bool) -> CheckResult {
    let root = project.root_path();

    let candidates = [
        "CHANGELOG.md",
        "CHANGELOG",
        "CHANGES.md",
        "CHANGES",
        "HISTORY.md",
        "HISTORY",
        "NEWS.md",
        "NEWS",
        "docs/CHANGELOG.md",
        "docs/changelog.md",
    ];

    let mut found: Option<String> = None;
    for name in candidates {
        let p = root.join(name);
        if p.is_file() {
            found = Some(name.to_string());
            break;
        }
    }

    let Some(changelog_name) = found else {
        return CheckResult::skip("changelog", "not found");
    };

    let path = root.join(&changelog_name);
    let Ok(content) = fs::read_to_string(&path) else {
        return CheckResult::warn("changelog", "unreadable");
    };

    // Heuristic: look for a recent version / Unreleased section
    let has_unreleased = content.to_lowercase().contains("[unreleased]")
        || content.to_lowercase().contains("## unreleased")
        || content.to_lowercase().contains("### unreleased");

    // Check if file was modified since last git tag
    let last_tag = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let changed_since_tag = if let Some(ref tag) = last_tag {
        Command::new("git")
            .args([
                "diff",
                "--name-only",
                &format!("{}..HEAD", tag),
                "--",
                &changelog_name,
            ])
            .current_dir(root)
            .output()
            .ok()
            .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
            .unwrap_or(false)
    } else {
        // No tags — just check if it has content beyond a header
        content.lines().filter(|l| !l.trim().is_empty()).count() > 3
    };

    if has_unreleased || changed_since_tag {
        CheckResult::pass_with("changelog", format!("{} (up to date)", changelog_name))
    } else if last_tag.is_some() {
        CheckResult::warn(
            "changelog",
            format!(
                "{} — may need update since {}",
                changelog_name,
                last_tag.unwrap()
            ),
        )
    } else {
        CheckResult::pass_with("changelog", changelog_name)
    }
}

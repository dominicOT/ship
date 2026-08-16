use crate::checks::CheckResult;
use crate::project::{self, Project, ProjectKind};
use std::process::Command;

pub fn run(project: &Project, _verbose: bool) -> CheckResult {
    let root = project.root_path();

    let version = match project.kind {
        ProjectKind::Rust => project::read_cargo_toml_version(root),
        ProjectKind::Node => project::read_package_json(root).and_then(|pkg| {
            pkg.get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        }),
        ProjectKind::Python => {
            // Try pyproject.toml
            let pyproject = root.join("pyproject.toml");
            if pyproject.exists() {
                if let Ok(content) = std::fs::read_to_string(&pyproject) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.starts_with("version") {
                            if let Some(v) = line.split('=').nth(1) {
                                return CheckResult::pass_with(
                                    "version",
                                    v.trim().trim_matches('"').to_string(),
                                );
                            }
                        }
                    }
                }
            }
            None
        }
        ProjectKind::Go => {
            // go.mod doesn't always have version; look for VERSION file or git
            None
        }
        ProjectKind::Unknown => None,
    };

    // Also check git describe / tags for consistency
    let git_version = Command::new("git")
        .args(["describe", "--tags", "--exact-match", "HEAD"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    match (version, git_version) {
        (Some(v), Some(g)) => {
            let clean_g = g.trim_start_matches('v');
            let clean_v = v.trim_start_matches('v');
            if clean_v == clean_g {
                CheckResult::pass_with("version", format!("{} (matches tag)", v))
            } else {
                CheckResult::warn("version", format!("package={} tag={}", v, g))
            }
        }
        (Some(v), None) => CheckResult::pass_with("version", v),
        (None, Some(g)) => CheckResult::pass_with("version", format!("git {}", g)),
        (None, None) => {
            // Last resort: any VERSION file
            for candidate in ["VERSION", "version.txt", "VERSION.txt"] {
                let p = root.join(candidate);
                if p.exists() {
                    if let Ok(content) = std::fs::read_to_string(&p) {
                        let v = content.trim().to_string();
                        if !v.is_empty() {
                            return CheckResult::pass_with("version", v);
                        }
                    }
                }
            }
            CheckResult::skip("version", "not found")
        }
    }
}

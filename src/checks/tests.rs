use crate::checks::CheckResult;
use crate::project::{Project, ProjectKind};
use crate::util::command_exists;
use std::process::Command;

pub fn run(project: &Project, verbose: bool) -> CheckResult {
    let root = project.root_path();

    let (cmd, args) = match project.kind {
        ProjectKind::Rust => ("cargo", vec!["test", "--quiet"]),
        ProjectKind::Node => {
            if root.join("pnpm-lock.yaml").exists() {
                ("pnpm", vec!["test"])
            } else if root.join("yarn.lock").exists() {
                ("yarn", vec!["test"])
            } else {
                ("npm", vec!["test", "--silent"])
            }
        }
        ProjectKind::Python => {
            if command_exists("pytest") {
                ("pytest", vec!["-q", "--tb=no"])
            } else {
                ("python", vec!["-m", "unittest", "discover", "-q"])
            }
        }
        ProjectKind::Go => ("go", vec!["test", "./..."]),
        ProjectKind::Unknown => {
            return CheckResult::skip("tests", "unknown project type");
        }
    };

    if !command_exists(cmd) {
        return CheckResult::skip("tests", format!("{} not found", cmd));
    }

    let mut command = Command::new(cmd);
    command.args(&args).current_dir(root);

    if matches!(project.kind, ProjectKind::Node) {
        command.env("CI", "true");
    }

    match command.output() {
        Ok(output) => {
            if output.status.success() {
                CheckResult::pass_with("tests", "passed")
            } else {
                if verbose {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let combined = format!("{}\n{}", stdout, stderr);
                    let snippet: String = combined
                        .lines()
                        .take(8)
                        .collect::<Vec<_>>()
                        .join("\n");
                    return CheckResult::fail("tests", "failed").with_extra(snippet);
                }
                CheckResult::fail("tests", "failed")
            }
        }
        Err(e) => CheckResult::fail("tests", format!("could not run: {}", e)),
    }
}

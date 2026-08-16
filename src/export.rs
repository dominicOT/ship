use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::checks::CheckResult;
use crate::project::Project;

#[derive(serde::Serialize)]
struct ExportProject {
    kind: String,
    root: Option<String>,
}

#[derive(serde::Serialize)]
struct ExportReport<'a> {
    project: ExportProject,
    results: &'a Vec<CheckResult>,
}

pub fn to_json_string(project: &Project, results: &Vec<CheckResult>) -> Result<String> {
    let exp_project = ExportProject {
        kind: project.kind.to_string(),
        root: project.root.as_ref().map(|p| p.display().to_string()),
    };

    let report = ExportReport {
        project: exp_project,
        results,
    };

    Ok(serde_json::to_string_pretty(&report)?)
}

pub fn write_json(path: &Path, project: &Project, results: &Vec<CheckResult>) -> Result<()> {
    let s = to_json_string(project, results)?;
    fs::write(path, s)?;
    Ok(())
}

pub fn to_markdown_string(project: &Project, results: &Vec<CheckResult>) -> String {
    let mut md = String::new();
    md.push_str("# ship report\n\n");
    md.push_str(&format!("**Project:** {}\n\n", project.kind));
    if let Some(ref root) = project.root {
        md.push_str(&format!("**Root:** {}\n\n", root.display()));
    }
    md.push_str("## Checks\n\n");

    for r in results {
        let status = match &r.status {
            crate::checks::CheckStatus::Pass => "pass",
            crate::checks::CheckStatus::Fail => "fail",
            crate::checks::CheckStatus::Warn => "warn",
            crate::checks::CheckStatus::Skip => "skip",
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

    md
}

pub fn write_markdown(path: &Path, project: &Project, results: &Vec<CheckResult>) -> Result<()> {
    let md = to_markdown_string(project, results);
    fs::write(path, md)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::CheckResult;
    // use crate::checks::CheckStatus;
    use crate::project::Project;
    use crate::project::ProjectKind;
    use std::path::PathBuf;

    #[test]
    fn json_serializes_project_and_results() {
        let project = Project {
            kind: ProjectKind::Rust,
            root: Some(PathBuf::from("/repo")),
        };
        let results = vec![CheckResult::pass_with("tests", "all good")];
        let s = to_json_string(&project, &results).expect("serialize");
        assert!(s.contains("project"));
        assert!(s.contains("tests"));
        assert!(s.contains("all good"));
    }

    #[test]
    fn markdown_contains_expected_sections() {
        let project = Project {
            kind: ProjectKind::Rust,
            root: Some(PathBuf::from("/repo")),
        };
        let results = vec![CheckResult::warn("todos", "2 found")];
        let md = to_markdown_string(&project, &results);
        assert!(md.contains("**Project:** Rust"));
        assert!(md.contains("## Checks"));
        assert!(md.contains("todos"));
        assert!(md.contains("2 found"));
    }
}

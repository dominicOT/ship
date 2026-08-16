use crate::checks::CheckResult;
use crate::project::{Project, ProjectKind};
use crate::util::walk_source_files;
use regex::Regex;
use std::fs;

pub fn run(project: &Project, verbose: bool) -> CheckResult {
    let root = project.root_path();

    let patterns: Vec<(&str, Regex)> = match project.kind {
        ProjectKind::Node | ProjectKind::Unknown => vec![
            (
                "console.log",
                Regex::new(r"\bconsole\.(log|debug|info|trace)\s*\(").unwrap(),
            ),
            ("debugger", Regex::new(r"\bdebugger\s*;").unwrap()),
        ],
        ProjectKind::Rust => vec![
            ("dbg!", Regex::new(r"\bdbg!\s*\(").unwrap()),
            (
                "debug println!",
                Regex::new(r#"(?i)\bprintln!\s*\(\s*".*(?:debug|todo|temp|FIXME|XXX)"#).unwrap(),
            ),
        ],
        ProjectKind::Python => vec![
            ("print(", Regex::new(r"\bprint\s*\(").unwrap()),
            (
                "breakpoint()",
                Regex::new(r"\bbreakpoint\s*\(\s*\)").unwrap(),
            ),
            (
                "pdb",
                Regex::new(r"\bimport\s+pdb\b|\bpdb\.set_trace\s*\(").unwrap(),
            ),
        ],
        ProjectKind::Go => vec![
            (
                "fmt.Print",
                Regex::new(r"\bfmt\.(Print|Printf|Println)\s*\(").unwrap(),
            ),
            (
                "log.Print",
                Regex::new(r"\blog\.(Print|Printf|Println)\s*\(").unwrap(),
            ),
        ],
    };

    let mut findings: Vec<String> = Vec::new();

    let text_exts = match project.kind {
        ProjectKind::Rust => vec!["rs"],
        ProjectKind::Node => vec!["js", "ts", "jsx", "tsx", "mjs", "cjs"],
        ProjectKind::Python => vec!["py"],
        ProjectKind::Go => vec!["go"],
        ProjectKind::Unknown => vec!["js", "ts", "jsx", "tsx", "py", "rs", "go"],
    };

    for entry in walk_source_files(root) {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.contains("test") || name.contains("spec") || name.starts_with("test_") {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !text_exts.iter().any(|e| *e == ext) {
            continue;
        }

        if path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s == "tests" || s == "test" || s == "__tests__"
        }) {
            continue;
        }

        if let Ok(content) = fs::read_to_string(path) {
            for (label, re) in &patterns {
                for (i, line) in content.lines().enumerate() {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("//")
                        || trimmed.starts_with('#')
                        || trimmed.starts_with("/*")
                    {
                        continue;
                    }
                    if re.is_match(line) {
                        let rel = path.strip_prefix(root).unwrap_or(path);
                        let snippet = line.trim();
                        let snippet = if snippet.len() > 70 {
                            format!("{}…", &snippet[..67])
                        } else {
                            snippet.to_string()
                        };
                        findings.push(format!(
                            "{}:{}  {} — {}",
                            rel.display(),
                            i + 1,
                            label,
                            snippet
                        ));
                        if findings.len() >= 25 {
                            break;
                        }
                    }
                }
                if findings.len() >= 25 {
                    break;
                }
            }
        }
        if findings.len() >= 25 {
            break;
        }
    }

    if findings.is_empty() {
        CheckResult::pass_with("console.logs", "none found")
    } else {
        let count = findings.len();
        let detail = format!("{} found", count);
        let extra = if verbose || count <= 8 {
            findings.join("\n")
        } else {
            format!("{}\n... and {} more", findings[..8].join("\n"), count - 8)
        };
        CheckResult::fail_soft("console.logs", detail).with_extra(extra)
    }
}

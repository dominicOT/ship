use crate::checks::CheckResult;
use crate::project::Project;
use crate::util::walk_source_files;
use regex::Regex;
use std::fs;

pub fn run(project: &Project, verbose: bool) -> CheckResult {
    let root = project.root_path();
    let re = Regex::new(r"(?i)\b(TODO|FIXME|XXX|HACK)\b[:\s]").unwrap();
    let mut findings: Vec<String> = Vec::new();

    let text_exts = [
        "rs", "js", "ts", "jsx", "tsx", "py", "go", "java", "kt", "rb", "php", "c", "cpp", "h",
        "hpp", "cs", "swift", "yml", "yaml", "toml", "md", "txt", "sh", "sql", "vue", "svelte",
        "graphql",
    ];

    for entry in walk_source_files(root) {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !ext.is_empty() && !text_exts.contains(&ext) {
            continue;
        }

        if let Ok(content) = fs::read_to_string(path) {
            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    let rel = path.strip_prefix(root).unwrap_or(path);
                    let snippet = line.trim();
                    let snippet = if snippet.len() > 80 {
                        format!("{}…", &snippet[..77])
                    } else {
                        snippet.to_string()
                    };
                    findings.push(format!("{}:{}  {}", rel.display(), i + 1, snippet));
                    if findings.len() >= 30 {
                        break;
                    }
                }
            }
        }
        if findings.len() >= 30 {
            break;
        }
    }

    if findings.is_empty() {
        CheckResult::pass_with("TODOs", "none found")
    } else {
        let count = findings.len();
        let detail = format!("{} found", count);
        let extra = if verbose || count <= 8 {
            findings.join("\n")
        } else {
            format!("{}\n... and {} more", findings[..8].join("\n"), count - 8)
        };
        CheckResult::fail_soft("TODOs", detail).with_extra(extra)
    }
}

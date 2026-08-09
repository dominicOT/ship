use crate::checks::CheckResult;
use crate::project::Project;
use crate::util::walk_source_files;
use regex::Regex;
use std::fs;

pub fn run(project: &Project, verbose: bool) -> CheckResult {
    let root = project.root_path();

    let patterns: Vec<(&str, Regex)> = vec![
        (
            "private key",
            Regex::new(r"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----").unwrap(),
        ),
        (
            "AWS access key",
            Regex::new(r"(?i)(?:AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}").unwrap(),
        ),
        (
            "GitHub token",
            Regex::new(r"(?i)\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,255}\b").unwrap(),
        ),
        (
            "OpenAI / Anthropic style key",
            Regex::new(r"(?i)\bsk-(?:ant-)?[A-Za-z0-9\-_]{20,}\b").unwrap(),
        ),
        (
            "Stripe key",
            Regex::new(r"(?i)\b(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{20,}\b").unwrap(),
        ),
        (
            "Slack token",
            Regex::new(r"(?i)\bxox[baprs]-[A-Za-z0-9-]{10,}\b").unwrap(),
        ),
        (
            "Google API key",
            Regex::new(r"(?i)\bAIza[0-9A-Za-z\-_]{35}\b").unwrap(),
        ),
        (
            "possible secret assignment",
            Regex::new(
                r#"(?i)(?:api[_-]?key|api[_-]?secret|access[_-]?token|auth[_-]?token|secret[_-]?key|private[_-]?key|password|passwd|credentials?)\s*[=:]\s*['"][A-Za-z0-9/\+=_\-\.]{16,}['"]"#
            )
            .unwrap(),
        ),
        (
            "db connection string",
            Regex::new(r#"(?i)(?:postgres|mysql|mongodb|redis)://[^:]+:[^@\s]{4,}@"#).unwrap(),
        ),
    ];

    let mut findings: Vec<String> = Vec::new();

    let skip_exts = [
        ".png", ".jpg", ".jpeg", ".gif", ".ico", ".svg", ".woff", ".woff2", ".ttf", ".eot",
        ".mp4", ".mp3", ".zip", ".tar", ".gz", ".lock", ".min.js", ".min.css",
    ];

    let text_exts = [
        "rs", "js", "ts", "jsx", "tsx", "py", "go", "java", "kt", "rb", "php", "c", "cpp",
        "h", "hpp", "cs", "swift", "env", "yml", "yaml", "toml", "json", "md", "txt",
        "sh", "bash", "zsh", "sql", "graphql", "vue", "svelte",
    ];

    for entry in walk_source_files(root) {
        let path = entry.path();

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if skip_exts.iter().any(|e| name.ends_with(e)) {
                continue;
            }
            if name.contains("example") || name.contains("sample") || name.contains("template") {
                continue;
            }
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let is_env = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with(".env"))
            .unwrap_or(false);
        if !ext.is_empty() && !text_exts.contains(&ext) && !is_env {
            continue;
        }

        if let Ok(content) = fs::read_to_string(path) {
            for (label, re) in &patterns {
                for mat in re.find_iter(&content) {
                    let rel = path.strip_prefix(root).unwrap_or(path);
                    let line_no = content[..mat.start()].lines().count();
                    findings.push(format!("{}:{} — {}", rel.display(), line_no, label));
                    if findings.len() >= 20 {
                        break;
                    }
                }
                if findings.len() >= 20 {
                    break;
                }
            }
        }
        if findings.len() >= 20 {
            break;
        }
    }

    if findings.is_empty() {
        CheckResult::pass_with("secrets", "none found")
    } else {
        let count = findings.len();
        let detail = format!("{} potential secret(s)", count);
        let extra = if verbose || count <= 5 {
            findings.join("\n")
        } else {
            format!("{}\n... and {} more", findings[..5].join("\n"), count - 5)
        };
        CheckResult::fail("secrets", detail).with_extra(extra)
    }
}

use crate::checks::CheckResult;
use crate::project::Project;
use crate::util::walk_source_files;
use regex::Regex;
use std::fs;
use std::sync::LazyLock;

/// Cap on total findings collected. High enough that a deliberate deep
/// scan won't silently stop early, but bounded so a pathological repo
/// can't make this run forever.
const MAX_FINDINGS: usize = 500;

struct Pattern {
    label: &'static str,
    regex: Regex,
}

fn pattern(label: &'static str, re: &str) -> Pattern {
    Pattern {
        label,
        regex: Regex::new(re).unwrap_or_else(|e| panic!("invalid regex for {label}: {e}")),
    }
}

static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    vec![
        // Private keys
        pattern(
            "private key",
            r"-----BEGIN (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY(?: BLOCK)?-----",
        ),
        // JWTs
        pattern(
            "JWT",
            r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b",
        ),
        pattern(
            "JWT signing secret",
            r#"(?i)\bjwt[_-]?(?:secret|signing[_-]?key)\b\s*[=:]\s*['"][^'"\s]{8,}['"]"#,
        ),
        // Cloud provider keys
        pattern(
            "AWS access key",
            r"(?i)(?:AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}",
        ),
        pattern(
            "Azure storage connection string",
            r"(?i)DefaultEndpointsProtocol=https?;AccountName=[^;]+;AccountKey=[A-Za-z0-9+/=]{20,}",
        ),
        pattern("Google API key", r"\bAIza[0-9A-Za-z\-_]{35}\b"),
        pattern("DigitalOcean token", r"\bdop_v1_[a-f0-9]{64}\b"),
        pattern("Docker Hub personal access token", r"\bdckr_pat_[A-Za-z0-9_-]{20,}\b"),
        pattern("npm access token", r"\bnpm_[A-Za-z0-9]{36}\b"),
        pattern("PyPI upload token", r"\bpypi-AgEIcHlwaS5vcmc[A-Za-z0-9_-]{20,}\b"),
        // Vendor / SaaS tokens
        pattern(
            "GitHub token",
            r"(?i)\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,255}\b",
        ),
        pattern(
            "OpenAI / Anthropic style key",
            r"(?i)\bsk-(?:ant-)?[A-Za-z0-9\-_]{20,}\b",
        ),
        pattern(
            "Stripe key",
            r"(?i)\b(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{20,}\b",
        ),
        pattern("Slack token", r"(?i)\bxox[baprs]-[A-Za-z0-9-]{10,}\b"),
        pattern("Twilio API key SID", r"\bSK[0-9a-fA-F]{32}\b"),
        pattern("SendGrid API key", r"\bSG\.[A-Za-z0-9_\-\.]{66}\b"),
        pattern("Mailgun API key", r"\bkey-[a-f0-9]{32}\b"),
        // Generic assignment / connection-string shapes
        pattern(
            "possible secret assignment",
            r#"(?i)(?:api[_-]?key|api[_-]?secret|access[_-]?token|auth[_-]?token|secret[_-]?key|private[_-]?key|password|passwd|credentials?)\s*[=:]\s*['"][A-Za-z0-9/\+=_\-\.]{16,}['"]"#,
        ),
        pattern(
            "db connection string",
            r#"(?i)(?:postgres|mysql|mariadb|mongodb|redis|mssql|sqlserver|amqps?)://[^:]+:[^@\s]{4,}@"#,
        ),
    ]
});

/// Filenames (exact, case-insensitive) scanned regardless of extension —
/// infra/CI files that commonly carry inline secrets.
const NAMED_TARGETS: &[&str] = &[
    "dockerfile",
    "docker-compose.yml",
    "docker-compose.yaml",
    "jenkinsfile",
    "makefile",
    "procfile",
];

const SKIP_EXTS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".svg", ".woff", ".woff2", ".ttf", ".eot", ".mp4",
    ".mp3", ".zip", ".tar", ".gz", ".lock", ".min.js", ".min.css",
];

const TEXT_EXTS: &[&str] = &[
    "rs", "js", "ts", "jsx", "tsx", "py", "go", "java", "kt", "rb", "php", "c", "cpp", "h", "hpp",
    "cs", "swift", "env", "yml", "yaml", "toml", "json", "md", "txt", "sh", "bash", "zsh", "sql",
    "graphql", "vue", "svelte", "tf", "tfvars", "hcl",
];

pub fn run(project: &Project, verbose: bool) -> CheckResult {
    let root = project.root_path();
    let mut findings: Vec<String> = Vec::new();

    'walk: for entry in walk_source_files(root) {
        let path = entry.path();

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let lower_name = file_name.to_ascii_lowercase();

        if SKIP_EXTS.iter().any(|e| lower_name.ends_with(e)) {
            continue;
        }
        if lower_name.contains("example") || lower_name.contains("sample") || lower_name.contains("template")
        {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let is_env = lower_name.starts_with(".env");
        let is_named_target = NAMED_TARGETS.iter().any(|n| lower_name == *n || lower_name.starts_with(&format!("{n}.")));

        if !ext.is_empty() && !TEXT_EXTS.contains(&ext.as_str()) && !is_env && !is_named_target {
            continue;
        }

        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };

        if let Some(label) = gcp_service_account_label(&lower_name, &content) {
            let rel = path.strip_prefix(root).unwrap_or(path);
            findings.push(format!("{}:1 — {}", rel.display(), label));
            if findings.len() >= MAX_FINDINGS {
                break 'walk;
            }
        }

        for pat in PATTERNS.iter() {
            for mat in pat.regex.find_iter(&content) {
                let rel = path.strip_prefix(root).unwrap_or(path);
                let line_no = content[..mat.start()].lines().count();
                findings.push(format!("{}:{} — {}", rel.display(), line_no, pat.label));
                if findings.len() >= MAX_FINDINGS {
                    break 'walk;
                }
            }
        }
    }

    if findings.is_empty() {
        CheckResult::pass_with("secrets", "none found")
    } else {
        let count = findings.len();
        let detail = format!("{} potential secret(s)", count);
        let shown = if verbose { 30 } else { 10 };
        let extra = if count <= shown {
            findings.join("\n")
        } else {
            format!(
                "{}\n... and {} more",
                findings[..shown].join("\n"),
                count - shown
            )
        };
        CheckResult::fail("secrets", detail).with_extra(extra)
    }
}

/// GCP service account key files are a common, high-value leak: flag
/// them specifically instead of relying on the generic private-key
/// pattern to label the finding usefully.
fn gcp_service_account_label(lower_name: &str, content: &str) -> Option<&'static str> {
    if !lower_name.ends_with(".json") {
        return None;
    }
    let is_service_account = content.contains("\"type\": \"service_account\"")
        || content.contains("\"type\":\"service_account\"");

    if is_service_account && content.contains("private_key") {
        return Some("GCP service account key");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn flags_jwt_secret_and_gcp_service_account_key() {
        let temp_dir =
            env::temp_dir().join(format!("ship_test_security_secrets_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create dir");

        fs::write(
            temp_dir.join("config.rb"),
            "jwt_secret = \"super-duper-secret-value\"",
        )
        .expect("write jwt secret file");

        fs::write(
            temp_dir.join("sa-key.json"),
            r#"{"type": "service_account", "private_key": "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----\n"}"#,
        )
        .expect("write gcp key file");

        fs::write(temp_dir.join("Dockerfile"), "ENV STRIPE_KEY sk_live_abcdefghijklmnopqrst")
            .expect("write dockerfile");

        let project = Project::detect_from(&temp_dir).expect("detect project");
        let result = run(&project, true);

        let extra = result.extra.clone().unwrap_or_default();
        assert!(extra.contains("JWT signing secret"), "extra: {extra}");
        assert!(extra.contains("GCP service account key"), "extra: {extra}");
        assert!(extra.contains("Stripe key"), "extra: {extra}");

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

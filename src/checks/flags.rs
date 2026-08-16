use crate::checks::CheckResult;
use crate::project::Project;
use crate::util::walk_source_files;
use regex::Regex;
use std::fs;

pub fn run(project: &Project, _verbose: bool) -> CheckResult {
    let root = project.root_path();

    let patterns: Vec<(&str, Regex)> = vec![
        (
            "temporary flag",
            Regex::new(r"(?i)(?:feature[_-]?flag|featureFlag|FF_|LAUNCH_DARKLY|unleash|flagsmith|configcat).*(?:TODO|FIXME|remove me|remove this)").unwrap(),
        ),
        (
            "hardcoded true flag",
            Regex::new(r#"(?i)(?:isEnabled|is_enabled|featureEnabled|feature_enabled|getFlag|get_flag)\s*\(\s*['"][^'"]+['"]\s*\)\s*(?:===?|==)\s*true"#).unwrap(),
        ),
        (
            "process.env feature",
            Regex::new(r#"(?i)process\.env\.(?:FEATURE_|FF_|ENABLE_|FLAG_)[A-Z0-9_]+"#).unwrap(),
        ),
    ];

    let mut findings: Vec<String> = Vec::new();
    let mut flag_mentions = 0;

    let text_exts = [
        "rs", "js", "ts", "jsx", "tsx", "py", "go", "java", "kt", "rb", "yml", "yaml", "toml",
        "json",
    ];

    for entry in walk_source_files(root) {
        let path = entry.path();
        // Skip our own checker to avoid self-matches on pattern strings
        if path.to_string_lossy().contains("checks/flags") {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !text_exts.contains(&ext) {
            continue;
        }

        if let Ok(content) = fs::read_to_string(path) {
            if content.to_lowercase().contains("feature flag")
                || content.contains("LaunchDarkly")
                || content.contains("unleash")
                || content.contains("flagsmith")
                || content.contains("configcat")
                || content.contains("posthog")
                || Regex::new(r"\bFF_[A-Z0-9_]+\b").unwrap().is_match(&content)
            {
                flag_mentions += 1;
            }

            for (label, re) in &patterns {
                for (i, line) in content.lines().enumerate() {
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
                        if findings.len() >= 15 {
                            break;
                        }
                    }
                }
            }
        }
        if findings.len() >= 15 {
            break;
        }
    }

    if findings.is_empty() {
        if flag_mentions > 0 {
            CheckResult::pass_with(
                "feature flags",
                format!("detected ({} files)", flag_mentions),
            )
        } else {
            CheckResult::skip("feature flags", "none detected")
        }
    } else {
        let detail = format!("{} suspicious", findings.len());
        let extra = findings.join("\n");
        CheckResult::warn("feature flags", detail).with_extra(extra)
    }
}

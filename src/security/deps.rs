use crate::checks::CheckResult;
use crate::project::{self, Project, ProjectKind};
use crate::util::version_lt;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// A small, hand-curated seed list of well-known, single-lineage
/// advisories (no separately-patched older major branches, so a plain
/// "resolved < fixed" comparison is safe). This is intentionally not
/// comprehensive — pair `ship security` with `cargo audit` / `npm
/// audit` / `pip-audit` for full coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ecosystem {
    Npm,
    Cargo,
    PyPi,
}

struct Advisory {
    ecosystem: Ecosystem,
    package: &'static str,
    fixed_version: &'static str,
    id: &'static str,
    summary: &'static str,
}

const ADVISORIES: &[Advisory] = &[
    Advisory {
        ecosystem: Ecosystem::Npm,
        package: "lodash",
        fixed_version: "4.17.21",
        id: "CVE-2021-23337",
        summary: "command injection via template()",
    },
    Advisory {
        ecosystem: Ecosystem::Npm,
        package: "minimist",
        fixed_version: "1.2.6",
        id: "CVE-2021-44906",
        summary: "prototype pollution",
    },
    Advisory {
        ecosystem: Ecosystem::Npm,
        package: "axios",
        fixed_version: "0.21.2",
        id: "CVE-2021-3749",
        summary: "ReDoS in trim()",
    },
    Advisory {
        ecosystem: Ecosystem::Npm,
        package: "node-fetch",
        fixed_version: "2.6.7",
        id: "CVE-2022-0235",
        summary: "sensitive header exposure on redirect",
    },
    Advisory {
        ecosystem: Ecosystem::PyPi,
        package: "PyYAML",
        fixed_version: "5.4",
        id: "CVE-2020-14343",
        summary: "arbitrary code execution via full_load/UnsafeLoader",
    },
    Advisory {
        ecosystem: Ecosystem::PyPi,
        package: "Pillow",
        fixed_version: "9.0.0",
        id: "CVE-2022-22817",
        summary: "buffer overflow in ImageMath.eval",
    },
    Advisory {
        ecosystem: Ecosystem::Cargo,
        package: "time",
        fixed_version: "0.2.23",
        id: "RUSTSEC-2020-0071",
        summary: "potential segfault from unsound localtime handling",
    },
    Advisory {
        ecosystem: Ecosystem::Cargo,
        package: "smallvec",
        fixed_version: "1.6.1",
        id: "RUSTSEC-2021-0003",
        summary: "buffer overflow in insert_many",
    },
];

/// Package name keywords treated as "security-sensitive" for the
/// loose-version-range check — auth, crypto, serialization, and web
/// framework packages carry more risk from an unpinned upgrade than a
/// loose range on, say, a formatting library.
const SENSITIVE_KEYWORDS: &[&str] = &[
    "auth", "jwt", "token", "oauth", "saml", "session", "password", "crypt", "bcrypt", "argon2",
    "ssl", "tls", "cors", "csrf", "helmet", "sanitize", "escape", "sql", "orm", "yaml", "pickle",
    "marshal", "deserialize", "express", "django", "flask", "rails", "actix", "rocket", "axum",
    "request", "axios", "fetch", "http",
];

fn is_security_sensitive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SENSITIVE_KEYWORDS.iter().any(|k| lower.contains(k))
}

pub fn run(project: &Project, verbose: bool) -> CheckResult {
    let root = project.root_path();

    let mut cve_findings: Vec<String> = Vec::new();
    let mut advisory_findings: Vec<String> = Vec::new();

    check_missing_lockfile(root, &project.kind, &mut advisory_findings);

    match project.kind {
        ProjectKind::Rust => {
            check_cargo_loose_ranges(root, &mut advisory_findings);
            if let Ok(content) = fs::read_to_string(root.join("Cargo.lock")) {
                let deps = parse_toml_package_blocks(&content);
                check_advisories(Ecosystem::Cargo, &deps, "Cargo.lock", &mut cve_findings);
            }
        }
        ProjectKind::Node => {
            check_node_loose_ranges(root, &mut advisory_findings);
            if let Some(deps) = parse_package_lock(root) {
                check_advisories(Ecosystem::Npm, &deps, "package-lock.json", &mut cve_findings);
            }
        }
        ProjectKind::Python => {
            check_poetry_loose_ranges(root, &mut advisory_findings);
            check_requirements_txt_unpinned(root, &mut advisory_findings);
            if let Ok(content) = fs::read_to_string(root.join("poetry.lock")) {
                let deps = parse_toml_package_blocks(&content);
                check_advisories(Ecosystem::PyPi, &deps, "poetry.lock", &mut cve_findings);
            }
        }
        ProjectKind::Go => {
            // go.sum parsing is wired up for future advisories; the
            // seed list doesn't include any Go modules yet.
            let _ = parse_go_sum(root);
        }
        ProjectKind::Unknown => {}
    }

    if cve_findings.is_empty() && advisory_findings.is_empty() {
        return CheckResult::pass_with("deps", "no known issues found");
    }

    let mut lines = cve_findings.clone();
    lines.extend(advisory_findings);

    let count = lines.len();
    let detail = format!("{count} issue(s) found");
    let shown = if verbose { 30 } else { 10 };
    let extra = if count <= shown {
        lines.join("\n")
    } else {
        format!(
            "{}\n... and {} more",
            lines[..shown].join("\n"),
            count - shown
        )
    };

    if !cve_findings.is_empty() {
        CheckResult::fail("deps", detail).with_extra(extra)
    } else {
        CheckResult::warn("deps", detail).with_extra(extra)
    }
}

fn check_advisories(
    ecosystem: Ecosystem,
    deps: &[(String, String)],
    lockfile_name: &str,
    findings: &mut Vec<String>,
) {
    for (name, version) in deps {
        for adv in ADVISORIES
            .iter()
            .filter(|a| a.ecosystem == ecosystem && a.package.eq_ignore_ascii_case(name))
        {
            if version_lt(version, adv.fixed_version) {
                findings.push(format!(
                    "{lockfile_name}: {name}@{version} — {} ({}), fixed in {}",
                    adv.id, adv.summary, adv.fixed_version
                ));
            }
        }
    }
}

// ---- lockfile parsing ----

/// Parses TOML `[[package]]` array-of-tables blocks shared by both
/// `Cargo.lock` and `poetry.lock`, pulling out each block's `name` and
/// `version` fields. Not a general TOML parser — just enough structure
/// to read these two specific, well-known lockfile shapes.
fn parse_toml_package_blocks(content: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut in_package = false;
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;

    let flush = |name: &mut Option<String>, version: &mut Option<String>, out: &mut Vec<(String, String)>| {
        if let (Some(n), Some(v)) = (name.take(), version.take()) {
            out.push((n, v));
        }
    };

    for line in content.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            flush(&mut name, &mut version, &mut result);
            in_package = true;
            continue;
        }
        if line.starts_with('[') {
            flush(&mut name, &mut version, &mut result);
            in_package = false;
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(rest) = line.strip_prefix("name") {
            if let Some(val) = extract_assigned_string(rest) {
                name = Some(val);
            }
        } else if let Some(rest) = line.strip_prefix("version") {
            if let Some(val) = extract_assigned_string(rest) {
                version = Some(val);
            }
        }
    }
    flush(&mut name, &mut version, &mut result);

    result
}

/// Given the text after a TOML key (e.g. ` = "1.2.3"`), extract the
/// quoted value if this really is a `key = "value"` assignment.
fn extract_assigned_string(after_key: &str) -> Option<String> {
    let rest = after_key.trim_start().strip_prefix('=')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_package_lock(root: &Path) -> Option<Vec<(String, String)>> {
    let content = fs::read_to_string(root.join("package-lock.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    let mut deps = Vec::new();

    if let Some(packages) = json.get("packages").and_then(|v| v.as_object()) {
        for (path, info) in packages {
            if path.is_empty() {
                continue;
            }
            let name = path.rsplit("node_modules/").next().unwrap_or(path);
            if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
                deps.push((name.to_string(), version.to_string()));
            }
        }
    } else if let Some(dependencies) = json.get("dependencies").and_then(|v| v.as_object()) {
        collect_v1_lock_deps(dependencies, &mut deps);
    }

    Some(deps)
}

fn collect_v1_lock_deps(obj: &serde_json::Map<String, serde_json::Value>, out: &mut Vec<(String, String)>) {
    for (name, info) in obj {
        if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
            out.push((name.clone(), version.to_string()));
        }
        if let Some(nested) = info.get("dependencies").and_then(|v| v.as_object()) {
            collect_v1_lock_deps(nested, out);
        }
    }
}

fn parse_go_sum(root: &Path) -> Option<Vec<(String, String)>> {
    let content = fs::read_to_string(root.join("go.sum")).ok()?;
    let mut seen = HashSet::new();
    let mut deps = Vec::new();

    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let (Some(module), Some(raw_version)) = (parts.next(), parts.next()) else {
            continue;
        };
        let version = raw_version.trim_end_matches("/go.mod");
        let key = (module.to_string(), version.to_string());
        if seen.insert(key.clone()) {
            deps.push(key);
        }
    }

    Some(deps)
}

// ---- missing lockfile ----

fn check_missing_lockfile(root: &Path, kind: &ProjectKind, findings: &mut Vec<String>) {
    match kind {
        ProjectKind::Rust => {
            if root.join("Cargo.toml").exists()
                && !root.join("Cargo.lock").exists()
                && cargo_is_binary(root)
            {
                findings.push(
                    "Cargo.lock missing — commit it for reproducible builds (binary crate)"
                        .to_string(),
                );
            }
        }
        ProjectKind::Node => {
            if root.join("package.json").exists()
                && !root.join("package-lock.json").exists()
                && !root.join("yarn.lock").exists()
                && !root.join("pnpm-lock.yaml").exists()
            {
                findings.push(
                    "No lockfile found (package-lock.json / yarn.lock / pnpm-lock.yaml)"
                        .to_string(),
                );
            }
        }
        ProjectKind::Python => {
            let pyproject = root.join("pyproject.toml");
            if let Ok(content) = fs::read_to_string(&pyproject) {
                if content.contains("[tool.poetry]") && !root.join("poetry.lock").exists() {
                    findings
                        .push("poetry.lock missing — commit it for reproducible builds".to_string());
                }
            }
        }
        ProjectKind::Go => {
            let go_mod = root.join("go.mod");
            if !root.join("go.sum").exists() {
                if let Ok(content) = fs::read_to_string(&go_mod) {
                    if content.lines().any(|l| l.trim_start().starts_with("require")) {
                        findings.push(
                            "go.sum missing — commit it for reproducible/verified builds"
                                .to_string(),
                        );
                    }
                }
            }
        }
        ProjectKind::Unknown => {}
    }
}

fn cargo_is_binary(root: &Path) -> bool {
    if root.join("src/main.rs").exists() {
        return true;
    }
    fs::read_to_string(root.join("Cargo.toml"))
        .map(|c| c.contains("[[bin]]"))
        .unwrap_or(false)
}

// ---- loose version ranges ----

fn check_node_loose_ranges(root: &Path, findings: &mut Vec<String>) {
    let Some(pkg) = project::read_package_json(root) else {
        return;
    };
    let Some(deps) = pkg.get("dependencies").and_then(|v| v.as_object()) else {
        return;
    };

    for (name, value) in deps {
        let Some(spec) = value.as_str() else {
            continue;
        };
        if is_loose_npm_range(spec) && is_security_sensitive(name) {
            findings.push(format!("package.json: {name} uses loose version range \"{spec}\""));
        }
    }
}

fn is_loose_npm_range(spec: &str) -> bool {
    let s = spec.trim();
    if s.is_empty() || s == "*" || s == "x" || s.eq_ignore_ascii_case("latest") {
        return true;
    }
    (s.starts_with(">=") || s.starts_with('>')) && !s.contains('<') && !s.contains("||")
}

fn check_cargo_loose_ranges(root: &Path, findings: &mut Vec<String>) {
    let Ok(content) = fs::read_to_string(root.join("Cargo.toml")) else {
        return;
    };

    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((name, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let rest = rest.trim();

        let spec = if let Some(stripped) = rest.strip_prefix('"') {
            stripped.split('"').next().map(str::to_string)
        } else if rest.starts_with('{') {
            extract_key_from_inline_table(rest, "version")
        } else {
            None
        };

        if let Some(spec) = spec {
            if spec.trim() == "*" && is_security_sensitive(name) {
                findings.push(format!("Cargo.toml: {name} uses loose version range \"*\""));
            }
        }
    }
}

fn check_poetry_loose_ranges(root: &Path, findings: &mut Vec<String>) {
    let Ok(content) = fs::read_to_string(root.join("pyproject.toml")) else {
        return;
    };

    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[tool.poetry.dependencies]";
            continue;
        }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((name, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim();
        let rest = rest.trim();

        let spec = if let Some(stripped) = rest.strip_prefix('"') {
            stripped.split('"').next().map(str::to_string)
        } else if rest.starts_with('{') {
            extract_key_from_inline_table(rest, "version")
        } else {
            None
        };

        if let Some(spec) = spec {
            if spec.trim() == "*" && is_security_sensitive(name) {
                findings.push(format!("pyproject.toml: {name} uses loose version range \"*\""));
            }
        }
    }
}

fn check_requirements_txt_unpinned(root: &Path, findings: &mut Vec<String>) {
    let Ok(content) = fs::read_to_string(root.join("requirements.txt")) else {
        return;
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('-') {
            continue;
        }
        let name = trimmed
            .split(|c: char| "=<>!~;[ ".contains(c))
            .next()
            .unwrap_or(trimmed);
        let has_pin = trimmed.len() > name.len();
        if !has_pin && is_security_sensitive(name) {
            findings.push(format!("requirements.txt: {name} has no version pin"));
        }
    }
}

fn extract_key_from_inline_table(inline: &str, key: &str) -> Option<String> {
    let idx = inline.find(key)?;
    let after = inline[idx + key.len()..].trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    let after = after.strip_prefix('"')?;
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn parses_cargo_lock_style_toml_blocks() {
        let toml = r#"
[[package]]
name = "time"
version = "0.2.20"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "serde"
version = "1.0.195"

[metadata]
"checksum time 0.2.20" = "deadbeef"
"#;
        let deps = parse_toml_package_blocks(toml);
        assert!(deps.contains(&("time".to_string(), "0.2.20".to_string())));
        assert!(deps.contains(&("serde".to_string(), "1.0.195".to_string())));
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn flags_known_vulnerable_cargo_dependency() {
        let toml = r#"
[[package]]
name = "time"
version = "0.2.20"

[[package]]
name = "serde"
version = "1.0.195"
"#;
        let deps = parse_toml_package_blocks(toml);
        let mut findings = Vec::new();
        check_advisories(Ecosystem::Cargo, &deps, "Cargo.lock", &mut findings);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].contains("RUSTSEC-2020-0071"));
    }

    #[test]
    fn does_not_flag_patched_versions() {
        let toml = r#"
[[package]]
name = "time"
version = "0.3.5"
"#;
        let deps = parse_toml_package_blocks(toml);
        let mut findings = Vec::new();
        check_advisories(Ecosystem::Cargo, &deps, "Cargo.lock", &mut findings);
        assert!(findings.is_empty());
    }

    #[test]
    fn parses_package_lock_v2_and_v1_shapes() {
        let temp_dir =
            env::temp_dir().join(format!("ship_test_deps_pkglock_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create dir");

        fs::write(
            temp_dir.join("package-lock.json"),
            r#"{
                "lockfileVersion": 3,
                "packages": {
                    "": {"name": "app"},
                    "node_modules/lodash": {"version": "4.17.15"},
                    "node_modules/foo/node_modules/lodash": {"version": "4.17.21"}
                }
            }"#,
        )
        .expect("write package-lock.json");

        let deps = parse_package_lock(&temp_dir).expect("parse");
        assert!(deps.contains(&("lodash".to_string(), "4.17.15".to_string())));
        assert!(deps.contains(&("lodash".to_string(), "4.17.21".to_string())));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn is_loose_npm_range_flags_wildcards_and_unbounded_ranges() {
        assert!(is_loose_npm_range("*"));
        assert!(is_loose_npm_range("latest"));
        assert!(is_loose_npm_range(">=1.0.0"));
        assert!(!is_loose_npm_range("^1.0.0"));
        assert!(!is_loose_npm_range("~1.2.3"));
        assert!(!is_loose_npm_range(">=1.0.0 <2.0.0"));
    }

    #[test]
    fn is_security_sensitive_matches_keywords_only() {
        assert!(is_security_sensitive("jsonwebtoken"));
        assert!(is_security_sensitive("express"));
        assert!(!is_security_sensitive("left-pad"));
    }
}

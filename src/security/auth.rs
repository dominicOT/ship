use crate::checks::CheckResult;
use crate::project::Project;
use crate::util::walk_source_files;
use regex::Regex;
use std::fs;
use std::sync::LazyLock;

/// Route/view path or class-name keywords that suggest a sensitive
/// endpoint worth double-checking for auth — not an exhaustive list,
/// just enough to keep the missing-middleware heuristic from firing on
/// every public route.
const SENSITIVE_KEYWORDS: &[&str] = &[
    "admin", "account", "user", "profile", "payment", "billing", "settings", "order", "invoice",
    "wallet", "delete", "password", "secret", "token", "credential", "internal",
];

fn looks_sensitive(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    SENSITIVE_KEYWORDS.iter().any(|k| lower.contains(k))
}

struct Regexes {
    jwt_none_alg: Regex,
    jwt_hardcoded_secret: Regex,
    jwt_verify_no_algorithms: Regex,
    express_route_no_middleware: Regex,
    flask_route: Regex,
    flask_auth_decorator: Regex,
}

static RE: LazyLock<Regexes> = LazyLock::new(|| Regexes {
    // `algorithm: "none"` / `algorithms: ["none"]` in JS or Python — the
    // classic JWT "alg:none" forgery vector.
    jwt_none_alg: Regex::new(r#"(?i)algorithms?\s*[:=]\s*(?:\[\s*)?['"]none['"]"#).unwrap(),
    // jwt.sign(payload, "literal-secret", ...) / jwt.encode(payload, "literal-secret", ...)
    // — a hardcoded signing secret passed directly at the call site,
    // not read from config/env.
    jwt_hardcoded_secret: Regex::new(
        r#"(?i)\bjwt\.(?:sign|verify|encode)\s*\([^)]*?,\s*['"][^'"\n]{4,}['"]"#,
    )
    .unwrap(),
    // jwt.verify(token, secret) / jwt.decode(token, secret) — captures
    // the whole call (up to its first closing paren) so the caller can
    // check whether `algorithms` appears anywhere inside it. The `regex`
    // crate has no look-around, so that check happens in Rust, not here.
    jwt_verify_no_algorithms: Regex::new(r#"(?i)\bjwt\.(?:verify|decode)\s*\([^)]*\)"#).unwrap(),
    // app.post('/path', (req, res) => ...) / router.get("/path", function(req, res) {
    // i.e. the token right after the path is itself a function, so
    // nothing (middleware) runs in between.
    express_route_no_middleware: Regex::new(
        r#"(?i)\b(?:app|router)\.(get|post|put|delete|patch|all)\(\s*['"]([^'"]+)['"]\s*,\s*(?:async\s+)?(?:function\b|\()"#,
    )
    .unwrap(),
    flask_route: Regex::new(r#"@(?:app|blueprint|bp)\.route\(\s*['"]([^'"]+)['"]"#).unwrap(),
    flask_auth_decorator: Regex::new(
        r"(?i)@\w*(?:login_required|jwt_required|auth_required|requires_auth|permission_required)\b",
    )
    .unwrap(),
});

const JS_EXTS: &[&str] = &["js", "jsx", "ts", "tsx", "mjs", "cjs"];

pub fn run(project: &Project, verbose: bool) -> CheckResult {
    let root = project.root_path();

    let mut critical: Vec<String> = Vec::new();
    let mut warn: Vec<String> = Vec::new();

    for entry in walk_source_files(root) {
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let is_js = JS_EXTS.contains(&ext.as_str());
        let is_py = ext == "py";

        if !is_js && !is_py {
            continue;
        }

        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        let rel = path.strip_prefix(root).unwrap_or(path);

        check_jwt_smells(&content, rel, &mut critical, &mut warn);

        if is_js {
            check_express_missing_middleware(&content, rel, &mut warn);
        }
        if is_py {
            check_flask_missing_auth(&content, rel, &mut warn);
        }
    }

    if critical.is_empty() && warn.is_empty() {
        return CheckResult::pass_with("auth", "no auth/JWT smells found");
    }

    let mut lines = critical.clone();
    lines.extend(warn);

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

    if !critical.is_empty() {
        CheckResult::fail("auth", detail).with_extra(extra)
    } else {
        CheckResult::warn("auth", detail).with_extra(extra)
    }
}

fn line_of(content: &str, byte_offset: usize) -> usize {
    content[..byte_offset].lines().count()
}

fn check_jwt_smells(
    content: &str,
    rel: &std::path::Path,
    critical: &mut Vec<String>,
    warn: &mut Vec<String>,
) {
    for mat in RE.jwt_none_alg.find_iter(content) {
        critical.push(format!(
            "{}:{} — JWT \"none\" algorithm allowed (accepts unsigned tokens)",
            rel.display(),
            line_of(content, mat.start())
        ));
    }

    for mat in RE.jwt_hardcoded_secret.find_iter(content) {
        critical.push(format!(
            "{}:{} — hardcoded secret passed directly to jwt.sign/verify/encode",
            rel.display(),
            line_of(content, mat.start())
        ));
    }

    for mat in RE.jwt_verify_no_algorithms.find_iter(content) {
        if !mat.as_str().to_ascii_lowercase().contains("algorithms") {
            warn.push(format!(
                "{}:{} — jwt.verify()/decode() call with no explicit `algorithms` allowlist",
                rel.display(),
                line_of(content, mat.start())
            ));
        }
    }
}

fn check_express_missing_middleware(content: &str, rel: &std::path::Path, warn: &mut Vec<String>) {
    for cap in RE.express_route_no_middleware.captures_iter(content) {
        let method = &cap[1];
        let route_path = &cap[2];
        if !looks_sensitive(route_path) {
            continue;
        }
        let mat = cap.get(0).unwrap();
        warn.push(format!(
            "{}:{} — {} {} has no middleware between path and handler (verify auth is enforced)",
            rel.display(),
            line_of(content, mat.start()),
            method.to_uppercase(),
            route_path
        ));
    }
}

fn check_flask_missing_auth(content: &str, rel: &std::path::Path, warn: &mut Vec<String>) {
    let lines: Vec<&str> = content.lines().collect();

    for cap in RE.flask_route.captures_iter(content) {
        let route_path = cap[1].to_string();
        if !looks_sensitive(&route_path) {
            continue;
        }
        let mat = cap.get(0).unwrap();
        let route_line = line_of(content, mat.start());

        // Look at the decorator stack directly around this @app.route
        // line (a couple lines above and below, since decorator order
        // varies) for a known auth decorator.
        let window_start = route_line.saturating_sub(4);
        let window_end = (route_line + 3).min(lines.len());
        let window = lines[window_start..window_end].join("\n");

        if !RE.flask_auth_decorator.is_match(&window) {
            warn.push(format!(
                "{}:{} — route \"{}\" has no login_required/auth decorator nearby",
                rel.display(),
                route_line,
                route_path
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn run_on(files: &[(&str, &str)]) -> CheckResult {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir =
            env::temp_dir().join(format!("ship_test_auth_{}_{id}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create dir");
        for (name, content) in files {
            fs::write(temp_dir.join(name), content).expect("write fixture");
        }
        let project = Project::detect_from(&temp_dir).expect("detect project");
        let result = run(&project, true);
        let _ = fs::remove_dir_all(&temp_dir);
        result
    }

    #[test]
    fn flags_none_algorithm() {
        let result = run_on(&[(
            "app.js",
            r#"jwt.verify(token, key, { algorithms: ["none"] });"#,
        )]);
        let extra = result.extra.unwrap_or_default();
        assert!(extra.contains("none\" algorithm"), "extra: {extra}");
    }

    #[test]
    fn flags_hardcoded_jwt_secret() {
        let result = run_on(&[(
            "app.js",
            r#"const token = jwt.sign({ id: 1 }, "super-secret-value-here");"#,
        )]);
        let extra = result.extra.unwrap_or_default();
        assert!(extra.contains("hardcoded secret"), "extra: {extra}");
    }

    #[test]
    fn flags_hardcoded_jwt_secret_with_multi_key_payload() {
        let result = run_on(&[(
            "app.js",
            r#"const token = jwt.sign({ id: 1, role: 'admin' }, "super-secret-value-here");"#,
        )]);
        let extra = result.extra.unwrap_or_default();
        assert!(extra.contains("hardcoded secret"), "extra: {extra}");
    }

    #[test]
    fn flags_verify_without_algorithms_allowlist() {
        let result = run_on(&[("app.js", r#"const decoded = jwt.verify(token, publicKey);"#)]);
        let extra = result.extra.unwrap_or_default();
        assert!(extra.contains("algorithms` allowlist"), "extra: {extra}");
    }

    #[test]
    fn does_not_flag_verify_with_algorithms_allowlist() {
        let result = run_on(&[(
            "app.js",
            r#"const decoded = jwt.verify(token, publicKey, { algorithms: ["RS256"] });"#,
        )]);
        assert_eq!(result.status, crate::checks::CheckStatus::Pass);
    }

    #[test]
    fn flags_express_sensitive_route_without_middleware() {
        let result = run_on(&[(
            "server.js",
            "app.post('/admin/users', (req, res) => { res.send('ok'); });\n",
        )]);
        let extra = result.extra.unwrap_or_default();
        assert!(extra.contains("POST /admin/users"), "extra: {extra}");
    }

    #[test]
    fn does_not_flag_express_route_with_middleware() {
        let result = run_on(&[(
            "server.js",
            "app.post('/admin/users', requireAuth, (req, res) => { res.send('ok'); });\n",
        )]);
        assert_eq!(result.status, crate::checks::CheckStatus::Pass);
    }

    #[test]
    fn does_not_flag_express_non_sensitive_route() {
        let result = run_on(&[(
            "server.js",
            "app.get('/health', (req, res) => { res.send('ok'); });\n",
        )]);
        assert_eq!(result.status, crate::checks::CheckStatus::Pass);
    }

    #[test]
    fn flags_flask_sensitive_route_without_login_required() {
        let result = run_on(&[(
            "views.py",
            "@app.route('/admin/dashboard')\ndef admin_dashboard():\n    return 'ok'\n",
        )]);
        let extra = result.extra.unwrap_or_default();
        assert!(extra.contains("admin/dashboard"), "extra: {extra}");
    }

    #[test]
    fn does_not_flag_flask_route_with_login_required() {
        let result = run_on(&[(
            "views.py",
            "@app.route('/admin/dashboard')\n@login_required\ndef admin_dashboard():\n    return 'ok'\n",
        )]);
        assert_eq!(result.status, crate::checks::CheckStatus::Pass);
    }
}

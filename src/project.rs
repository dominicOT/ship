use anyhow::Result;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectKind {
    Rust,
    Node,
    Python,
    Go,
    Unknown,
}

impl std::fmt::Display for ProjectKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectKind::Rust => write!(f, "Rust"),
            ProjectKind::Node => write!(f, "Node.js"),
            ProjectKind::Python => write!(f, "Python"),
            ProjectKind::Go => write!(f, "Go"),
            ProjectKind::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug)]
pub struct Project {
    pub kind: ProjectKind,
    pub root: Option<PathBuf>,
}

impl Project {
    pub fn detect() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let root = find_project_root(&cwd);

        let kind = if let Some(ref r) = root {
            detect_kind(r)
        } else {
            detect_kind(&cwd)
        };

        Ok(Project {
            kind,
            root: root.or(Some(cwd)),
        })
    }

    pub fn root_path(&self) -> &Path {
        self.root.as_deref().unwrap_or(Path::new("."))
    }
}

fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("Cargo.toml").exists()
            || current.join("package.json").exists()
            || current.join("pyproject.toml").exists()
            || current.join("setup.py").exists()
            || current.join("go.mod").exists()
            || current.join(".git").exists()
        {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn detect_kind(root: &Path) -> ProjectKind {
    if root.join("Cargo.toml").exists() {
        ProjectKind::Rust
    } else if root.join("package.json").exists() {
        ProjectKind::Node
    } else if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("requirements.txt").exists()
    {
        ProjectKind::Python
    } else if root.join("go.mod").exists() {
        ProjectKind::Go
    } else {
        ProjectKind::Unknown
    }
}

pub fn read_package_json(root: &Path) -> Option<serde_json::Value> {
    let path = root.join("package.json");
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn read_cargo_toml_version(root: &Path) -> Option<String> {
    let path = root.join("Cargo.toml");
    let content = fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("version") {
            if let Some(v) = line.split('=').nth(1) {
                return Some(v.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

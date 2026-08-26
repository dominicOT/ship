use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

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
    #[allow(dead_code)]
    pub fn detect() -> Result<Self> {
        Self::detect_from_path(None)
    }

    #[allow(dead_code)]
    pub fn detect_from(path: &Path) -> Result<Self> {
        Self::detect_from_path(Some(path))
    }

    pub fn detect_from_path(path: Option<&Path>) -> Result<Self> {
        let base_path = match path {
            Some(p) => {
                if !p.exists() {
                    anyhow::bail!("Project directory does not exist: {}", p.display());
                }
                if !p.is_dir() {
                    anyhow::bail!("Project path is not a directory: {}", p.display());
                }
                p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
            }
            None => std::env::current_dir()?,
        };

        let root = find_project_root(&base_path);

        let kind = if let Some(ref r) = root {
            detect_kind(r)
        } else {
            detect_kind(&base_path)
        };

        Ok(Project {
            kind,
            root: root.or(Some(base_path)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn detect_from_current_dir() {
        let project = Project::detect().expect("detect current project");
        assert_eq!(project.kind, ProjectKind::Rust);
        assert!(project.root.is_some());
    }

    #[test]
    fn detect_from_valid_custom_path() {
        let temp_dir = std::env::temp_dir().join(format!("ship_test_project_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        File::create(temp_dir.join("package.json")).expect("create package.json");

        let project = Project::detect_from(&temp_dir).expect("detect project");
        assert_eq!(project.kind, ProjectKind::Node);
        assert_eq!(
            project.root_path(),
            temp_dir.canonicalize().unwrap_or(temp_dir.clone())
        );

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn detect_from_nonexistent_path_fails() {
        let nonexistent = Path::new("/path/that/does/not/exist_ship_12345");
        let result = Project::detect_from(nonexistent);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not exist"));
    }

    #[test]
    fn detect_from_file_fails() {
        let temp_file = std::env::temp_dir().join(format!("ship_test_file_{}", std::process::id()));
        File::create(&temp_file).expect("create file");

        let result = Project::detect_from(&temp_file);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not a directory"));

        let _ = fs::remove_file(&temp_file);
    }
}

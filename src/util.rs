use std::env;
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

pub fn command_exists(cmd: &str) -> bool {
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let p = dir.join(cmd);
            if p.is_file() {
                return true;
            }
        }
    }
    false
}

pub fn walk_source_files(root: &Path) -> impl Iterator<Item = DirEntry> {
    let skip_dirs = [
        "node_modules",
        "target",
        "dist",
        "build",
        ".git",
        "vendor",
        "__pycache__",
        ".venv",
        "venv",
        ".next",
        ".nuxt",
        "coverage",
        "Pods",
        ".idea",
        ".vscode",
    ];

    WalkDir::new(root)
        .into_iter()
        .filter_entry(move |e| {
            let name = e.file_name().to_string_lossy();

            // Exclude generated report files
            if name == "ship-report.md" || name == "ship-report.json" {
                return false;
            }

            if e.file_type().is_dir() {
                !skip_dirs.iter().any(|d| name == *d) && !name.starts_with('.')
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
}

use std::path::Path;
use std::env;
use walkdir::{WalkDir, DirEntry};

pub fn command_exists(cmd: &str) -> bool {
    if let Ok(path) = env::var("PATH") {
        for dir in path.split(':') {
            let p = Path::new(dir).join(cmd);
            if p.is_file() {
                return true;
            }
        }
    }
    false
}

pub fn walk_source_files(root: &Path) -> impl Iterator<Item = DirEntry> {
    let skip_dirs = [
        "node_modules", "target", "dist", "build", ".git", "vendor",
        "__pycache__", ".venv", "venv", ".next", ".nuxt", "coverage",
        "Pods", ".idea", ".vscode",
    ];

    WalkDir::new(root)
        .into_iter()
        .filter_entry(move |e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                !skip_dirs.iter().any(|d| name == *d) && !name.starts_with('.')
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
}

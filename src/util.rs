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
        "generated",
        "__generated__",
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn walk_source_files_skips_generated_dirs() {
        let temp_dir =
            env::temp_dir().join(format!("ship_test_walk_generated_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join("src/generated/prisma")).expect("create dirs");
        fs::write(temp_dir.join("src/main.rs"), "fn main() {}").expect("write file");
        fs::write(
            temp_dir.join("src/generated/prisma/client.ts"),
            "AKIAABCDEFGHIJKLMNOP",
        )
        .expect("write generated file");

        let files: Vec<_> = walk_source_files(&temp_dir)
            .map(|e| e.path().to_path_buf())
            .collect();

        assert!(files.iter().any(|p| p.ends_with("src/main.rs")));
        assert!(!files
            .iter()
            .any(|p| p.ends_with("src/generated/prisma/client.ts")));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

use ignore::{DirEntry, WalkBuilder};
use std::env;
use std::path::Path;

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

    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .ignore(true)
        .filter_entry(move |e| {
            let name = e.file_name().to_string_lossy();

            // Exclude generated report files
            if name == "ship-report.md" || name == "ship-report.json" {
                return false;
            }

            if e.file_type().is_some_and(|t| t.is_dir()) {
                !skip_dirs.iter().any(|d| name == *d) && !name.starts_with('.')
            } else {
                true
            }
        });

    builder
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
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

    #[test]
    fn walk_source_files_respects_gitignore_and_ignore_files() {
        let temp_dir =
            env::temp_dir().join(format!("ship_test_walk_ignore_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create dir");

        fs::write(temp_dir.join("main.rs"), "fn main() {}").expect("write file");
        fs::write(temp_dir.join(".env"), "SECRET=abc").expect("write .env");
        fs::write(temp_dir.join("scratch.log"), "debug output").expect("write scratch.log");
        fs::write(temp_dir.join(".gitignore"), ".env\n").expect("write .gitignore");
        fs::write(temp_dir.join(".ignore"), "scratch.log\n").expect("write .ignore");

        let files: Vec<_> = walk_source_files(&temp_dir)
            .map(|e| e.path().to_path_buf())
            .collect();

        assert!(files.iter().any(|p| p.ends_with("main.rs")));
        assert!(!files.iter().any(|p| p.ends_with(".env")));
        assert!(!files.iter().any(|p| p.ends_with("scratch.log")));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

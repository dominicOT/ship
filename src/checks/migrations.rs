use crate::checks::CheckResult;
use crate::project::{Project, ProjectKind};
use std::fs;

pub fn run(project: &Project, _verbose: bool) -> CheckResult {
    let root = project.root_path();

    // Common migration locations
    let candidates = [
        // Prisma
        "prisma/migrations",
        // Django
        "migrations",
        // Rails
        "db/migrate",
        // Diesel (Rust)
        "migrations",
        // Flyway / Liquibase style
        "src/main/resources/db/migration",
        "db/migration",
        "db/migrations",
        // Knex / Sequelize / TypeORM
        "migrations",
        "src/migrations",
        "database/migrations",
        // Alembic (Python)
        "alembic/versions",
        // Goose / golang-migrate
        "migrations",
        "db/migrations",
    ];

    let mut found_dirs: Vec<&str> = Vec::new();
    let mut migration_count = 0usize;

    for rel in candidates {
        let p = root.join(rel);
        if p.is_dir() {
            if let Ok(entries) = fs::read_dir(&p) {
                let count = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        // skip .gitkeep etc
                        !name.starts_with('.') && (e.path().is_file() || e.path().is_dir())
                    })
                    .count();
                if count > 0 {
                    found_dirs.push(rel);
                    migration_count += count;
                }
            }
        }
    }

    // Also check for migration tools in package manifests
    let has_tool = match project.kind {
        ProjectKind::Node => {
            if let Some(pkg) = crate::project::read_package_json(root) {
                let deps = ["prisma", "knex", "typeorm", "sequelize", "drizzle-orm", "mikro-orm"];
                let all_deps = pkg
                    .get("dependencies")
                    .into_iter()
                    .chain(pkg.get("devDependencies"))
                    .filter_map(|d| d.as_object())
                    .flat_map(|m| m.keys().cloned())
                    .collect::<Vec<_>>();
                deps.iter().any(|d| all_deps.iter().any(|k| k.contains(d)))
            } else {
                false
            }
        }
        ProjectKind::Rust => {
            // diesel, sqlx, sea-orm in Cargo.toml
            if let Ok(content) = fs::read_to_string(root.join("Cargo.toml")) {
                content.contains("diesel") || content.contains("sqlx") || content.contains("sea-orm")
            } else {
                false
            }
        }
        ProjectKind::Python => {
            root.join("alembic.ini").exists()
                || root.join("manage.py").exists() // Django
        }
        _ => false,
    };

    if found_dirs.is_empty() && !has_tool {
        CheckResult::skip("migrations", "none detected")
    } else if found_dirs.is_empty() && has_tool {
        CheckResult::pass_with("migrations", "tool present, no migration files")
    } else {
        // We can't easily know if they are "pending" without running the tool,
        // so we just report presence. Encourage review.
        let dirs = found_dirs.join(", ");
        CheckResult::pass_with(
            "migrations",
            format!("{} file(s) in {}", migration_count, dirs),
        )
    }
}

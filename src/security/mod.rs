pub mod auth;
pub mod deps;
pub mod env_files;
pub mod secrets;

pub const ALL_CHECKS: &[&str] = &["secrets", "env-files", "deps", "auth"];

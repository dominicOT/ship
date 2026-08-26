use std::fs;
use std::path::Path;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ShipConfig {
    pub skip: Vec<String>,
    pub only: Vec<String>,
}

impl ShipConfig {
    pub fn load_from_dir(root: &Path) -> Option<Self> {
        let config_path = root.join(".ship.toml");
        if !config_path.is_file() {
            return None;
        }

        let content = fs::read_to_string(config_path).ok()?;
        Some(Self::parse(&content))
    }

    pub fn parse(content: &str) -> Self {
        let mut config = ShipConfig::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim();
                let val = val.trim();

                match key {
                    "skip" => {
                        config.skip = parse_string_list(val);
                    }
                    "only" => {
                        config.only = parse_string_list(val);
                    }
                    _ => {}
                }
            }
        }

        config
    }
}

fn parse_string_list(val: &str) -> Vec<String> {
    let trimmed = val.trim().trim_start_matches('[').trim_end_matches(']');
    trimmed
        .split(',')
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_with_skip_and_only() {
        let content = r#"
            # This is a comment
            skip = ["tests", "migrations"]
            only = ["secrets"]
        "#;
        let config = ShipConfig::parse(content);
        assert_eq!(config.skip, vec!["tests", "migrations"]);
        assert_eq!(config.only, vec!["secrets"]);
    }

    #[test]
    fn parse_empty_config() {
        let content = "# All commented\n# skip = [\"tests\"]\n";
        let config = ShipConfig::parse(content);
        assert!(config.skip.is_empty());
        assert!(config.only.is_empty());
    }
}

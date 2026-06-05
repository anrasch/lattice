use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub roots: Vec<PathBuf>,
    #[serde(default = "default_ignore_file")]
    pub ignore_file: String,
    #[serde(default = "default_db_path")]
    pub db_path: PathBuf,
    #[serde(default = "default_ref_fields")]
    pub ref_fields: Vec<String>,
}

fn default_ignore_file() -> String {
    ".aiignore".to_string()
}

fn default_db_path() -> PathBuf {
    PathBuf::from("lattice.db")
}

fn default_ref_fields() -> Vec<String> {
    vec!["related".to_string()]
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_config() {
        let c: Config = toml::from_str(
            "roots = [\"./vault\"]\nignore_file = \".aiignore\"\ndb_path = \"lattice.db\"\n",
        )
        .unwrap();
        assert_eq!(c.roots, vec![PathBuf::from("./vault")]);
        assert_eq!(c.ignore_file, ".aiignore");
    }

    #[test]
    fn applies_defaults() {
        let c: Config = toml::from_str("roots = [\".\"]\n").unwrap();
        assert_eq!(c.ignore_file, ".aiignore");
        assert_eq!(c.db_path, PathBuf::from("lattice.db"));
        assert_eq!(c.ref_fields, vec!["related".to_string()]);
    }
}

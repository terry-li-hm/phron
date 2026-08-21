use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub vault: VaultConfig,
    pub llm: LlmConfig,
    pub research: ResearchConfig,
    pub thresholds: ThresholdsConfig,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct VaultConfig {
    pub path: String,
    pub overnight_dir: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct LlmConfig {
    pub synthesis_model: String,
    pub research_model: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ResearchConfig {
    pub topics: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct ThresholdsConfig {
    pub health_red: u32,
    pub health_yellow: u32,
}

fn parse_config(contents: &str) -> Result<Config> {
    toml::from_str(contents).context("Failed to parse config.toml format")
}

fn load_config_from(path: &Path) -> Result<Config> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file at {:?}", path))?;
    parse_config(&contents)
}

pub fn load_config() -> Result<Config> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let config_path = home.join(".config").join("comes").join("config.toml");
    load_config_from(&config_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const VALID: &str = r#"
[vault]
path = "~/notes"
overnight_dir = "Daily Intelligence"

[llm]
synthesis_model = "claude-sonnet-4-6"
research_model = "google/gemini-flash-1.5"

[research]
topics = ["topic-a", "topic-b"]

[thresholds]
health_red = 50
health_yellow = 70
"#;

    #[test]
    fn parse_valid_config() {
        let config = parse_config(VALID).unwrap();
        assert_eq!(config.vault.path, "~/notes");
        assert_eq!(config.vault.overnight_dir, "Daily Intelligence");
        assert_eq!(config.llm.synthesis_model, "claude-sonnet-4-6");
        assert_eq!(config.llm.research_model, "google/gemini-flash-1.5");
        assert_eq!(config.research.topics, vec!["topic-a", "topic-b"]);
        assert_eq!(config.thresholds.health_red, 50);
        assert_eq!(config.thresholds.health_yellow, 70);
    }

    #[test]
    fn parse_repo_example_config() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.toml.example");
        let contents = fs::read_to_string(path).unwrap();
        let config = parse_config(&contents).unwrap();
        assert_eq!(config.thresholds.health_red, 50);
        assert_eq!(config.thresholds.health_yellow, 70);
        assert!(!config.research.topics.is_empty());
    }

    #[test]
    fn empty_topics_is_valid() {
        let toml = VALID.replace("topics = [\"topic-a\", \"topic-b\"]", "topics = []");
        let config = parse_config(&toml).unwrap();
        assert!(config.research.topics.is_empty());
    }

    #[test]
    fn extra_unknown_fields_are_ignored() {
        let toml = format!("{VALID}\nunused = true\n[extra]\nfoo = 1\n");
        parse_config(&toml).unwrap();
    }

    #[test]
    fn malformed_toml_errors() {
        let err = parse_config("this is not toml {{{{").unwrap_err();
        assert!(err
            .to_string()
            .contains("Failed to parse config.toml format"));
    }

    #[test]
    fn empty_string_errors() {
        assert!(parse_config("").is_err());
    }

    #[test]
    fn missing_section_errors() {
        let toml = r#"
[vault]
path = "~/notes"
overnight_dir = "Daily Intelligence"
"#;
        assert!(parse_config(toml).is_err());
    }

    #[test]
    fn missing_required_field_errors() {
        let toml = r#"
[vault]
path = "~/notes"
overnight_dir = "Daily Intelligence"

[llm]
synthesis_model = "x"

[research]
topics = []

[thresholds]
health_red = 50
"#;
        assert!(parse_config(toml).is_err());
    }

    #[test]
    fn wrong_type_errors() {
        let toml = VALID.replace("health_red = 50", "health_red = \"low\"");
        assert!(parse_config(&toml).is_err());
    }

    #[test]
    fn zero_thresholds_are_accepted() {
        let toml = VALID
            .replace("health_red = 50", "health_red = 0")
            .replace("health_yellow = 70", "health_yellow = 0");
        let config = parse_config(&toml).unwrap();
        assert_eq!(config.thresholds.health_red, 0);
        assert_eq!(config.thresholds.health_yellow, 0);
    }

    #[test]
    fn load_from_missing_file_errors() {
        let dir = crate::test_support::temp_dir();
        let path = dir.join("missing.toml");
        let err = load_config_from(&path).unwrap_err();
        assert!(err.to_string().contains("Failed to read config file"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_from_temp_file_roundtrip() {
        let dir = crate::test_support::temp_dir();
        let path = dir.join("config.toml");
        fs::write(&path, VALID).unwrap();
        let config = load_config_from(&path).unwrap();
        assert_eq!(config.thresholds.health_yellow, 70);
        let _ = fs::remove_dir_all(dir);
    }
}

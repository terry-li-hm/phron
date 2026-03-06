use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub vault: VaultConfig,
    pub llm: LlmConfig,
    pub research: ResearchConfig,
    pub thresholds: ThresholdsConfig,
}

#[derive(Debug, Deserialize)]
pub struct VaultConfig {
    pub path: String,
    pub overnight_dir: String,
}

#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub synthesis_model: String,
    pub research_model: String,
}

#[derive(Debug, Deserialize)]
pub struct ResearchConfig {
    pub topics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ThresholdsConfig {
    pub health_red: u32,
    pub health_yellow: u32,
}

pub fn load_config() -> Result<Config> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let config_path = home.join(".config").join("comes").join("config.toml");

    let contents = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file at {:?}", config_path))?;

    let config: Config = toml::from_str(&contents).context("Failed to parse config.toml format")?;

    Ok(config)
}

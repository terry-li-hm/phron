use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct State {
    pub last_overnight_run: Option<NaiveDate>,
    pub health_history: Vec<HealthEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthEntry {
    pub date: NaiveDate,
    pub score: u32,
}

fn get_state_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let path = home.join(".config").join("comes").join("state.json");
    Ok(path)
}

pub fn load_state() -> Result<State> {
    let path = get_state_path()?;
    if !path.exists() {
        return Ok(State::default());
    }

    let contents = fs::read_to_string(&path)?;
    // If parse fails, return default to avoid crashing
    let state = serde_json::from_str(&contents).unwrap_or_else(|_| State::default());
    Ok(state)
}

pub fn save_state(state: &State) -> Result<()> {
    let path = get_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(state)?;
    fs::write(&path, json)?;
    Ok(())
}

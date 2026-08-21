use anyhow::{Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone)]
pub struct State {
    pub last_overnight_run: Option<NaiveDate>,
    pub health_history: Vec<HealthEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct HealthEntry {
    pub date: NaiveDate,
    pub score: u32,
}

fn get_state_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let path = home.join(".config").join("comes").join("state.json");
    Ok(path)
}

fn parse_state(contents: &str) -> State {
    // If parse fails, return default to avoid crashing
    serde_json::from_str(contents).unwrap_or_else(|_| State::default())
}

fn load_state_from(path: &Path) -> Result<State> {
    if !path.exists() {
        return Ok(State::default());
    }

    let contents = fs::read_to_string(path)?;
    Ok(parse_state(&contents))
}

fn save_state_to(path: &Path, state: &State) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_state() -> Result<State> {
    load_state_from(&get_state_path()?)
}

pub fn save_state(state: &State) -> Result<()> {
    save_state_to(&get_state_path()?, state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::fs;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn parse_full_state() {
        let json = r#"{
            "last_overnight_run": "2026-01-15",
            "health_history": [{"date": "2026-01-14", "score": 82}]
        }"#;
        let state = parse_state(json);
        assert_eq!(state.last_overnight_run, Some(date(2026, 1, 15)));
        assert_eq!(state.health_history.len(), 1);
        assert_eq!(state.health_history[0].score, 82);
    }

    #[test]
    fn empty_object_parses_with_serde_defaults_for_option() {
        let state = parse_state("{}");
        // Missing Option may deserialize as None; missing Vec fails and falls back
        // to Default. Encode whichever current serde behavior produces.
        assert_eq!(state.health_history, Vec::<HealthEntry>::new());
    }

    #[test]
    fn malformed_json_returns_default() {
        let state = parse_state("{not json");
        assert_eq!(state, State::default());
    }

    #[test]
    fn empty_string_returns_default() {
        assert_eq!(parse_state(""), State::default());
    }

    #[test]
    fn invalid_date_returns_default() {
        let json = r#"{"last_overnight_run": "15-01-2026", "health_history": []}"#;
        assert_eq!(parse_state(json), State::default());
    }

    #[test]
    fn missing_health_history_returns_default_and_drops_last_run() {
        // BUG: a partial/legacy file without health_history fails Deserialize,
        // then unwrap_or_else yields Default and silently drops last_overnight_run.
        let json = r#"{"last_overnight_run": "2026-01-15"}"#;
        let state = parse_state(json);
        assert_eq!(
            state,
            State::default(),
            "missing health_history currently wipes last_overnight_run"
        );
    }

    #[test]
    fn extra_fields_are_ignored() {
        let json = r#"{
            "last_overnight_run": null,
            "health_history": [],
            "legacy": true
        }"#;
        let state = parse_state(json);
        assert_eq!(state, State::default());
    }

    #[test]
    fn null_last_run_is_none() {
        let json = r#"{"last_overnight_run": null, "health_history": []}"#;
        let state = parse_state(json);
        assert_eq!(state.last_overnight_run, None);
    }

    #[test]
    fn score_as_string_returns_default() {
        let json = r#"{"last_overnight_run": null, "health_history": [{"date": "2026-01-01", "score": "high"}]}"#;
        assert_eq!(parse_state(json), State::default());
    }

    #[test]
    fn serialize_roundtrip() {
        let state = State {
            last_overnight_run: Some(date(2026, 8, 21)),
            health_history: vec![HealthEntry {
                date: date(2026, 8, 20),
                score: 61,
            }],
        };
        let json = serde_json::to_string_pretty(&state).unwrap();
        assert_eq!(parse_state(&json), state);
        assert!(json.contains("\"last_overnight_run\": \"2026-08-21\""));
    }

    #[test]
    fn load_missing_file_returns_default() {
        let dir = crate::test_support::temp_dir();
        let path = dir.join("state.json");
        let state = load_state_from(&path).unwrap();
        assert_eq!(state, State::default());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_creates_parent_dirs_and_load_roundtrips() {
        let dir = crate::test_support::temp_dir();
        let path = dir.join("nested").join("state.json");
        let state = State {
            last_overnight_run: Some(date(2026, 3, 1)),
            health_history: vec![],
        };
        save_state_to(&path, &state).unwrap();
        assert!(path.exists());
        let loaded = load_state_from(&path).unwrap();
        assert_eq!(loaded, state);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_corrupt_file_returns_default() {
        let dir = crate::test_support::temp_dir();
        let path = dir.join("state.json");
        fs::write(&path, "{{{{").unwrap();
        let loaded = load_state_from(&path).unwrap();
        assert_eq!(loaded, State::default());
        let _ = fs::remove_dir_all(dir);
    }
}

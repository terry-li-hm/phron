use anyhow::Result;
use chrono::Local;
use owo_colors::OwoColorize;

use crate::clients::oura::OuraClient;
use crate::config::Config;
use crate::state::State;

pub fn generate_health_report(config: &Config) -> Result<String> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let oura = OuraClient::new()?;
    let readiness_data = oura.daily_readiness(&today)?;
    let sleep_data = oura.daily_sleep(&today)?;

    let score = readiness_data.first().map(|r| r.score).unwrap_or(0);

    let hrv = readiness_data
        .first()
        .and_then(|r| r.contributors.hrv_balance)
        .unwrap_or(0);

    let sleep = sleep_data.first().map(|s| s.score).unwrap_or(0);

    let state_text;
    let recommendation;
    let dot = "●";

    if score >= config.thresholds.health_yellow {
        state_text = format!("{} GREEN ({})", dot, score);
        recommendation = "Push through, you're sharp";
    } else if score >= config.thresholds.health_red {
        state_text = format!("{} YELLOW ({})", dot, score);
        recommendation = "Moderate your energy today";
    } else {
        state_text = format!("{} RED ({})", dot, score);
        recommendation = "Protect your energy. Reschedule heavy tasks.";
    }

    Ok(format!(
        "{} — HRV {}ms · Sleep {} · {}",
        state_text, hrv, sleep, recommendation
    ))
}

pub fn run(config: &Config, _state: &State) -> Result<()> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let oura = OuraClient::new()?;
    let readiness_data = oura.daily_readiness(&today)?;
    let sleep_data = oura.daily_sleep(&today)?;

    let score = readiness_data.first().map(|r| r.score).unwrap_or(0);

    let hrv = readiness_data
        .first()
        .and_then(|r| r.contributors.hrv_balance)
        .unwrap_or(0);

    let sleep = sleep_data.first().map(|s| s.score).unwrap_or(0);

    let state_text;
    let recommendation;
    let dot = "●";

    if score >= config.thresholds.health_yellow {
        state_text = dot.green().to_string() + &format!(" GREEN ({})", score).green().to_string();
        recommendation = "Push through, you're sharp";
    } else if score >= config.thresholds.health_red {
        state_text =
            dot.yellow().to_string() + &format!(" YELLOW ({})", score).yellow().to_string();
        recommendation = "Moderate your energy today";
    } else {
        state_text = dot.red().to_string() + &format!(" RED ({})", score).red().to_string();
        recommendation = "Protect your energy. Reschedule heavy tasks.";
    }

    println!(
        "{} — HRV {}ms · Sleep {} · {}",
        state_text, hrv, sleep, recommendation
    );

    Ok(())
}

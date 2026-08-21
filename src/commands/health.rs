use anyhow::Result;
use chrono::Local;
use owo_colors::OwoColorize;

use crate::clients::oura::OuraClient;
use crate::config::Config;
use crate::state::State;

fn fetch_metrics(today: &str) -> Result<(u32, u32, u32)> {
    let oura = OuraClient::new()?;
    let readiness_data = oura.daily_readiness(today)?;
    let sleep_data = oura.daily_sleep(today)?;

    let score = readiness_data.first().map(|r| r.score).unwrap_or(0);
    let hrv = readiness_data
        .first()
        .and_then(|r| r.contributors.hrv_balance)
        .unwrap_or(0);
    let sleep = sleep_data.first().map(|s| s.score).unwrap_or(0);

    Ok((score, hrv, sleep))
}

pub fn generate_health_report(config: &Config) -> Result<String> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let (score, hrv, sleep) = match fetch_metrics(&today) {
        Ok(metrics) => metrics,
        Err(e) => {
            return Ok(format!("Health data unavailable — Oura error: {}", e));
        }
    };

    Ok(format_health_report(
        score,
        hrv,
        sleep,
        config.thresholds.health_yellow,
        config.thresholds.health_red,
    ))
}

fn format_health_report(score: u32, hrv: u32, sleep: u32, yellow: u32, red: u32) -> String {
    let state_text;
    let recommendation;
    let dot = "●";

    if score >= yellow {
        state_text = format!("{} GREEN ({})", dot, score);
        recommendation = "Push through, you're sharp";
    } else if score >= red {
        state_text = format!("{} YELLOW ({})", dot, score);
        recommendation = "Moderate your energy today";
    } else {
        state_text = format!("{} RED ({})", dot, score);
        recommendation = "Protect your energy. Reschedule heavy tasks.";
    }

    format!(
        "{} — HRV {}ms · Sleep {} · {}",
        state_text, hrv, sleep, recommendation
    )
}

pub fn run(config: &Config, _state: &State) -> Result<()> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let (score, hrv, sleep) = match fetch_metrics(&today) {
        Ok(metrics) => metrics,
        Err(e) => {
            eprintln!("Oura data unavailable: {}", e);
            println!("Health data unavailable — readiness and sleep could not be loaded.");
            return Ok(());
        }
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    const YELLOW: u32 = 70;
    const RED: u32 = 50;

    #[test]
    fn green_at_yellow_threshold() {
        let line = format_health_report(70, 60, 80, YELLOW, RED);
        assert!(line.contains("GREEN (70)"));
        assert!(line.contains("Push through, you're sharp"));
        assert!(line.contains("HRV 60ms"));
        assert!(line.contains("Sleep 80"));
    }

    #[test]
    fn yellow_just_below_green() {
        let line = format_health_report(69, 1, 1, YELLOW, RED);
        assert!(line.contains("YELLOW (69)"));
        assert!(line.contains("Moderate your energy today"));
    }

    #[test]
    fn yellow_at_red_threshold() {
        let line = format_health_report(50, 0, 0, YELLOW, RED);
        assert!(line.contains("YELLOW (50)"));
    }

    #[test]
    fn red_just_below_red_threshold() {
        let line = format_health_report(49, 10, 10, YELLOW, RED);
        assert!(line.contains("RED (49)"));
        assert!(line.contains("Protect your energy. Reschedule heavy tasks."));
    }

    #[test]
    fn zero_score_is_red_not_unavailable() {
        // fetch_metrics maps missing Oura rows to 0, and this formatter then
        // prints RED rather than "unavailable".
        let line = format_health_report(0, 0, 0, YELLOW, RED);
        assert!(line.contains("RED (0)"));
        assert!(!line.to_lowercase().contains("unavailable"));
    }

    #[test]
    fn labels_hrv_as_milliseconds() {
        // Current formatting treats Oura hrv_balance (a 1-100 score) as ms.
        let line = format_health_report(80, 42, 70, YELLOW, RED);
        assert!(line.contains("HRV 42ms"));
    }

    #[test]
    fn equal_thresholds_prefer_green() {
        let line = format_health_report(50, 0, 0, 50, 50);
        assert!(line.contains("GREEN (50)"));
    }

    #[test]
    fn inverted_thresholds_never_yield_yellow() {
        // If yellow < red, any score >= yellow hits GREEN first.
        let line = format_health_report(60, 0, 0, 50, 70);
        assert!(line.contains("GREEN (60)"));
        assert!(!line.contains("YELLOW"));
    }

    #[test]
    fn max_score_is_green() {
        let line = format_health_report(u32::MAX, u32::MAX, u32::MAX, YELLOW, RED);
        assert!(line.contains("GREEN"));
    }
}

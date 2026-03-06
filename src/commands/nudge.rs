use anyhow::Result;
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;

use crate::clients::oura::OuraClient;
use crate::clients::telegram::TelegramClient;
use crate::config::Config;

fn log_action(msg: &str) {
    if let Some(home) = dirs::home_dir() {
        let log_path = home.join("logs").join("comes-nudge.log");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
        }
    }
}

pub fn run(config: &Config) -> Result<()> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let oura = match OuraClient::new() {
        Ok(client) => client,
        Err(e) => {
            log_action(&format!("Failed to init Oura client: {}", e));
            return Ok(()); // LaunchAgent must exit 0
        }
    };

    let readiness_data = match oura.daily_readiness(&today) {
        Ok(data) => data,
        Err(e) => {
            log_action(&format!("Failed to fetch Oura readiness: {}", e));
            return Ok(());
        }
    };

    let score = readiness_data.first().map(|r| r.score).unwrap_or(0);

    if score > 0 && score < config.thresholds.health_red {
        log_action(&format!(
            "Red state detected (score: {}). Triggering nudges.",
            score
        ));

        // 1. Due reminder
        let mut cmd = Command::new("moneo");
        cmd.arg("add")
            .arg("Energy low today — reschedule heavy tasks")
            .arg("--in")
            .arg("5m")
            .arg("--sync");

        match cmd.output() {
            Ok(out) if out.status.success() => log_action("Added moneo due reminder successfully"),
            Ok(out) => log_action(&format!(
                "Moneo failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )),
            Err(e) => log_action(&format!("Failed to run moneo: {}", e)),
        }

        // 2. Telegram message
        if let Ok(tg) = TelegramClient::new() {
            let msg = format!(
                "🔴 Low readiness today (score: {}). Protect your energy.",
                score
            );
            if let Err(e) = tg.send_message(&msg) {
                log_action(&format!("Failed to send Telegram nudge: {}", e));
            } else {
                log_action("Sent Telegram nudge successfully");
            }
        } else {
            log_action("Failed to init Telegram client");
        }
    } else {
        log_action(&format!(
            "Readiness is {} (not Red). No nudge needed.",
            score
        ));
    }

    Ok(())
}

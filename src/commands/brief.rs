use anyhow::Result;
use chrono::Local;
use std::process::Command;

use crate::clients::llm::LlmClient;
use crate::clients::oura::OuraClient;
use crate::clients::vault::VaultClient;
use crate::config::Config;
use crate::state::State;

pub fn generate_brief(config: &Config, state: &mut State) -> Result<String> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    // 1. Health state
    let oura = OuraClient::new()?;
    let score = match oura.daily_readiness(&today) {
        Ok(data) => data.first().map(|r| r.score).unwrap_or(0),
        Err(_) => 0, // Fallback if Oura fails
    };

    let health_context = if score >= config.thresholds.health_yellow {
        format!("Health score is {} (Green) - feeling sharp.", score)
    } else if score >= config.thresholds.health_red {
        format!("Health score is {} (Yellow) - moderate energy.", score)
    } else if score > 0 {
        format!(
            "Health score is {} (Red) - low energy, protect yourself.",
            score
        )
    } else {
        "Health data unavailable.".to_string()
    };

    // 2. Calendar intensity
    let calendar_out = Command::new("fasti").arg("list").arg("--today").output();

    let calendar_context = match calendar_out {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => "Calendar empty or unavailable.".to_string(),
    };

    // 3. Digest context (if ran today)
    let vault = VaultClient::new(&config.vault.path)?;
    let mut digest_context = String::new();

    if let Some(last_run) = state.last_overnight_run {
        if last_run.format("%Y-%m-%d").to_string() == today {
            if let Ok(Some(content)) = vault.read_digest(&config.vault.overnight_dir, &today) {
                let snippet = content.chars().take(500).collect::<String>();
                digest_context = format!(
                    "Overnight digest snippet:
{}",
                    snippet
                );
            }
        }
    }

    // 4. Synthesize with LLM
    let llm = LlmClient::new()?;
    let prompt = format!(
        "Synthesize a 150-word morning brief for a professional based on this context. Be direct, authoritative, and practical. Act as a life coach.

Health: {}

Calendar:
{}

{}",
        health_context, calendar_context, digest_context
    );

    let brief = llm.generate(
        &config.llm.synthesis_model,
        &prompt,
        Some("You are an executive life coach."),
    )?;

    Ok(brief)
}

pub fn run(config: &Config, state: &mut State) -> Result<()> {
    let brief = generate_brief(config, state)?;

    println!(
        "
=== Morning Brief ===
"
    );
    println!("{}", brief);
    println!(
        "
=====================
"
    );

    Ok(())
}

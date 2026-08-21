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
    let score = match OuraClient::new() {
        Ok(oura) => match oura.daily_readiness(&today) {
            Ok(data) => data.first().map(|r| r.score).unwrap_or(0),
            Err(_) => 0,
        },
        Err(_) => 0,
    };

    let health_context = health_context(
        score,
        config.thresholds.health_yellow,
        config.thresholds.health_red,
    );

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
                let snippet = digest_snippet(&content, 500);
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

fn health_context(score: u32, yellow: u32, red: u32) -> String {
    if score >= yellow {
        format!("Health score is {} (Green) - feeling sharp.", score)
    } else if score >= red {
        format!("Health score is {} (Yellow) - moderate energy.", score)
    } else if score > 0 {
        format!(
            "Health score is {} (Red) - low energy, protect yourself.",
            score
        )
    } else {
        "Health data unavailable.".to_string()
    }
}

fn digest_snippet(content: &str, max_chars: usize) -> String {
    content.chars().take(max_chars).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    const YELLOW: u32 = 70;
    const RED: u32 = 50;

    #[test]
    fn green_context() {
        assert_eq!(
            health_context(70, YELLOW, RED),
            "Health score is 70 (Green) - feeling sharp."
        );
    }

    #[test]
    fn yellow_context() {
        assert_eq!(
            health_context(50, YELLOW, RED),
            "Health score is 50 (Yellow) - moderate energy."
        );
    }

    #[test]
    fn red_context_for_positive_score() {
        assert_eq!(
            health_context(1, YELLOW, RED),
            "Health score is 1 (Red) - low energy, protect yourself."
        );
    }

    #[test]
    fn zero_score_is_unavailable_unlike_health_report() {
        assert_eq!(health_context(0, YELLOW, RED), "Health data unavailable.");
    }

    #[test]
    fn digest_snippet_short_content_unchanged() {
        assert_eq!(digest_snippet("hello", 500), "hello");
    }

    #[test]
    fn digest_snippet_empty() {
        assert_eq!(digest_snippet("", 500), "");
    }

    #[test]
    fn digest_snippet_truncates_on_chars_not_bytes() {
        let content: String = "你".repeat(600);
        let snippet = digest_snippet(&content, 500);
        assert_eq!(snippet.chars().count(), 500);
        assert!(snippet.ends_with('你'));
    }

    #[test]
    fn digest_snippet_keeps_emoji_intact() {
        let content = "😀".repeat(10);
        assert_eq!(digest_snippet(&content, 3), "😀😀😀");
    }

    #[test]
    fn digest_snippet_zero_max_is_empty() {
        assert_eq!(digest_snippet("abc", 0), "");
    }
}

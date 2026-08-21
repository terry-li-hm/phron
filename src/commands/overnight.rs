use anyhow::Result;
use chrono::Local;

use crate::clients::llm::LlmClient;
use crate::clients::openrouter::OpenRouterClient;
use crate::clients::telegram::TelegramClient;
use crate::clients::vault::VaultClient;
use crate::config::Config;
use crate::state::State;

fn already_ran_today(last_run: Option<chrono::NaiveDate>, today: chrono::NaiveDate) -> bool {
    last_run == Some(today)
}

fn preview_digest(final_digest: &str) -> String {
    if final_digest.len() > 2000 {
        format!(
            "{}...

[Truncated, full digest in Vault]",
            &final_digest[..2000]
        )
    } else {
        final_digest.to_string()
    }
}

fn send_alert_and_exit(msg: &str) -> ! {
    if let Ok(tg) = TelegramClient::new() {
        let _ = tg.send_message(msg);
    }
    eprintln!("{}", msg);
    std::process::exit(1);
}

pub fn run(config: &Config, state: &mut State) -> Result<()> {
    let today = Local::now().date_naive();

    if already_ran_today(state.last_overnight_run, today) {
        println!("Overnight agent already ran today. Exiting.");
        return Ok(());
    }

    let openrouter = OpenRouterClient::new().unwrap_or_else(|e| {
        send_alert_and_exit(&format!(
            "⚠️ overnight research failed: OpenRouter client error: {}",
            e
        ));
    });

    let llm = LlmClient::new().unwrap_or_else(|e| {
        send_alert_and_exit(&format!(
            "⚠️ overnight research failed: Anthropic client error: {}",
            e
        ));
    });

    let vault = VaultClient::new(&config.vault.path).unwrap_or_else(|e| {
        send_alert_and_exit(&format!(
            "⚠️ overnight research failed: Vault client error: {}",
            e
        ));
    });

    let tg = TelegramClient::new().unwrap_or_else(|e| {
        send_alert_and_exit(&format!(
            "⚠️ overnight research failed: Telegram client error: {}",
            e
        ));
    });

    let mut raw_findings = Vec::new();

    println!("Starting overnight research...");

    for topic in &config.research.topics {
        println!("Researching topic: {}", topic);
        let prompt = format!("Provide a detailed research briefing on recent developments for: {}. Include concrete facts, recent news, and strategic implications.", topic);

        match openrouter.generate(&config.llm.research_model, &prompt) {
            Ok(result) => {
                raw_findings.push(format!(
                    "### Topic: {}

{}",
                    topic, result
                ));
            }
            Err(e) => {
                send_alert_and_exit(&format!(
                    "⚠️ overnight research failed: API error on topic {}: {}",
                    topic, e
                ));
            }
        }
    }

    println!("Synthesizing digest...");

    let combined_research = raw_findings.join(
        "

---

",
    );
    let synthesis_prompt = format!(
        "You are an expert executive analyst. Synthesize the following research findings into a highly structured, well-formatted daily digest. Focus on actionable insights, strategic moves, and major updates.

Research material:
{}",
        combined_research
    );

    let final_digest = match llm.generate(
        &config.llm.synthesis_model,
        &synthesis_prompt,
        Some("You write high-signal intelligence briefs."),
    ) {
        Ok(digest) => digest,
        Err(e) => send_alert_and_exit(&format!(
            "⚠️ overnight research failed: Synthesis error: {}",
            e
        )),
    };

    let date_str = today.format("%Y-%m-%d").to_string();

    if let Err(e) =
        vault.write_overnight_digest(&config.vault.overnight_dir, &date_str, &final_digest)
    {
        send_alert_and_exit(&format!(
            "⚠️ overnight research failed: Vault write error: {}",
            e
        ));
    }

    let preview = preview_digest(&final_digest);

    let tg_message = format!(
        "<b>🌙 Overnight Intelligence Digest - {}</b>

{}",
        date_str, preview
    );
    if let Err(e) = tg.send_message(&tg_message) {
        eprintln!("Warning: failed to send Telegram message: {}", e);
    }

    state.last_overnight_run = Some(today);
    if let Err(e) = crate::state::save_state(state) {
        eprintln!("Warning: failed to save state: {}", e);
    }

    println!("Overnight research complete.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn already_ran_when_same_day() {
        let today = date(2026, 8, 21);
        assert!(already_ran_today(Some(today), today));
    }

    #[test]
    fn not_already_ran_when_different_day() {
        assert!(!already_ran_today(
            Some(date(2026, 8, 20)),
            date(2026, 8, 21)
        ));
    }

    #[test]
    fn not_already_ran_when_never() {
        assert!(!already_ran_today(None, date(2026, 8, 21)));
    }

    #[test]
    fn preview_leaves_short_digest_unchanged() {
        assert_eq!(preview_digest("short"), "short");
        assert_eq!(preview_digest(""), "");
    }

    #[test]
    fn preview_does_not_truncate_exactly_2000_bytes() {
        let digest = "a".repeat(2000);
        assert_eq!(preview_digest(&digest), digest);
    }

    #[test]
    fn preview_truncates_ascii_over_2000_bytes() {
        let digest = "a".repeat(2001);
        let preview = preview_digest(&digest);
        assert!(preview.starts_with(&"a".repeat(2000)));
        assert!(preview.contains("[Truncated, full digest in Vault]"));
        assert!(preview.contains("..."));
    }

    #[test]
    fn preview_panics_when_byte_2000_is_not_a_char_boundary() {
        // BUG: preview_digest slices at byte 2000. A 3-byte char starting at
        // index 1999 makes 2000 a non-boundary and panics.
        let mut digest = "a".repeat(1999);
        digest.push('你');
        assert!(digest.len() > 2000);
        assert!(!digest.is_char_boundary(2000));

        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| preview_digest(&digest)));
        assert!(
            result.is_err(),
            "expected panic when truncating through a multibyte character"
        );
    }
}

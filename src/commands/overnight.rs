use anyhow::Result;
use chrono::Local;

use crate::clients::llm::LlmClient;
use crate::clients::openrouter::OpenRouterClient;
use crate::clients::telegram::TelegramClient;
use crate::clients::vault::VaultClient;
use crate::config::Config;
use crate::state::State;

fn send_alert_and_exit(msg: &str) -> ! {
    if let Ok(tg) = TelegramClient::new() {
        let _ = tg.send_message(msg);
    }
    eprintln!("{}", msg);
    std::process::exit(1);
}

pub fn run(config: &Config, state: &mut State) -> Result<()> {
    let today = Local::now().date_naive();

    if let Some(last_run) = state.last_overnight_run {
        if last_run == today {
            println!("Overnight agent already ran today. Exiting.");
            return Ok(());
        }
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

    let preview = if final_digest.len() > 2000 {
        format!(
            "{}...

[Truncated, full digest in Vault]",
            &final_digest[..2000]
        )
    } else {
        final_digest.clone()
    };

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

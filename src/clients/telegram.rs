use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TelegramResponse {
    ok: bool,
    description: Option<String>,
}

pub struct TelegramClient {
    client: Client,
    bot_token: String,
    chat_id: String,
}

impl TelegramClient {
    pub fn new() -> Result<Self> {
        let bot_token =
            std::env::var("TELEGRAM_BOT_TOKEN").context("TELEGRAM_BOT_TOKEN not set")?;
        let chat_id = std::env::var("TELEGRAM_CHAT_ID").context("TELEGRAM_CHAT_ID not set")?;
        Ok(Self {
            client: Client::new(),
            bot_token,
            chat_id,
        })
    }

    pub fn send_message(&self, text: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let resp = self
            .client
            .post(&url)
            .form(&[
                ("chat_id", &self.chat_id as &str),
                ("text", text),
                ("parse_mode", "HTML"),
            ])
            .send()
            .context("Failed to reach Telegram API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            bail!("Telegram API returned {status}: {body}");
        }

        let tg_resp: TelegramResponse = resp.json()?;
        if !tg_resp.ok {
            bail!(
                "Telegram returned ok=false: {}",
                tg_resp.description.unwrap_or_default()
            );
        }

        Ok(())
    }
}

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
        require_telegram_ok(tg_resp)
    }
}

fn require_telegram_ok(tg_resp: TelegramResponse) -> Result<()> {
    if !tg_resp.ok {
        bail!(
            "Telegram returned ok=false: {}",
            tg_resp.description.unwrap_or_default()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_true_succeeds() {
        let resp: TelegramResponse = serde_json::from_str(r#"{"ok": true}"#).unwrap();
        require_telegram_ok(resp).unwrap();
    }

    #[test]
    fn ok_false_with_description_errors() {
        let resp: TelegramResponse =
            serde_json::from_str(r#"{"ok": false, "description": "Forbidden"}"#).unwrap();
        let err = require_telegram_ok(resp).unwrap_err();
        assert!(err.to_string().contains("Forbidden"));
        assert!(err.to_string().contains("ok=false"));
    }

    #[test]
    fn ok_false_without_description_uses_empty_string() {
        let resp: TelegramResponse = serde_json::from_str(r#"{"ok": false}"#).unwrap();
        let err = require_telegram_ok(resp).unwrap_err();
        assert_eq!(err.to_string(), "Telegram returned ok=false: ");
    }

    #[test]
    fn extra_fields_are_ignored() {
        let resp: TelegramResponse =
            serde_json::from_str(r#"{"ok": true, "result": {"message_id": 1}}"#).unwrap();
        require_telegram_ok(resp).unwrap();
    }

    #[test]
    fn missing_ok_field_errors() {
        let parsed: Result<TelegramResponse, _> = serde_json::from_str(r#"{"description": "x"}"#);
        assert!(parsed.is_err());
    }

    #[test]
    fn malformed_json_errors() {
        let parsed: Result<TelegramResponse, _> = serde_json::from_str("not-json");
        assert!(parsed.is_err());
    }
}

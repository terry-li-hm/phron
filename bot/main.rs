use anyhow::{Context, Result};
use phron::commands::{health, brief};
use phron::config;
use phron::state;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::env;
use std::fs;
use std::thread;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct Update {
    update_id: i64,
    message: Option<Message>,
}

#[derive(Debug, Deserialize)]
struct Message {
    chat: Chat,
    text: Option<String>,
    voice: Option<Voice>,
}

#[derive(Debug, Deserialize)]
struct Chat {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct Voice {
    file_id: String,
}

#[derive(Debug, Deserialize)]
struct TgResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct File {
    file_path: Option<String>,
}

struct Bot {
    client: Client,
    token: String,
    offset: i64,
}

impl Bot {
    fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
            offset: 0,
        }
    }

    fn get_updates(&mut self) -> Result<Vec<Update>> {
        let url = format!("https://api.telegram.org/bot{}/getUpdates", self.token);
        let offset_str = self.offset.to_string();
        let resp = self.client.get(&url)
            .query(&[
                ("offset", offset_str.as_str()),
                ("timeout", "30"),
            ])
            .send()
            .context("Failed to reach Telegram API")?;

        if !resp.status().is_success() {
            anyhow::bail!("Telegram API returned error status: {}", resp.status());
        }

        let body: TgResponse<Vec<Update>> = resp.json().context("Failed to parse updates")?;
        if !body.ok {
            anyhow::bail!("Telegram returned ok=false: {}", body.description.unwrap_or_default());
        }

        let updates = body.result.unwrap_or_default();
        if let Some(last) = updates.last() {
            self.offset = last.update_id + 1;
        }

        Ok(updates)
    }

    fn send_message(&self, chat_id: i64, text: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let chat_id_str = chat_id.to_string();
        let resp = self.client.post(&url)
            .form(&[
                ("chat_id", chat_id_str.as_str()),
                ("text", text),
                ("parse_mode", "HTML"),
            ])
            .send()
            .context("Failed to send message")?;

        if !resp.status().is_success() {
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("Failed to send message: {}", body);
        }

        Ok(())
    }

    fn download_voice(&self, file_id: &str) -> Result<()> {
        // 1. Get file path
        let url = format!("https://api.telegram.org/bot{}/getFile", self.token);
        let resp = self.client.get(&url)
            .query(&[("file_id", file_id)])
            .send()
            .context("Failed to call getFile")?;
        
        let body: TgResponse<File> = resp.json().context("Failed to parse getFile response")?;
        if !body.ok || body.result.is_none() {
            anyhow::bail!("Failed to get file path: {}", body.description.unwrap_or_default());
        }

        let file_path = body.result.unwrap().file_path.context("No file path in getFile response")?;
        
        // 2. Download file
        let download_url = format!("https://api.telegram.org/file/bot{}/{}", self.token, file_path);
        let mut resp = self.client.get(&download_url).send().context("Failed to download voice file")?;
        
        let target_path = format!("/tmp/comes-audio-{}.ogg", file_id);
        let mut file = fs::File::create(&target_path).context("Failed to create local voice file")?;
        resp.copy_to(&mut file).context("Failed to save voice file")?;
        
        Ok(())
    }
}

fn main() {
    let token = env::var("TELEGRAM_COMES_BOT_TOKEN")
        .or_else(|_| env::var("TELEGRAM_BOT_TOKEN"))
        .expect("Neither TELEGRAM_COMES_BOT_TOKEN nor TELEGRAM_BOT_TOKEN is set");

    let mut bot = Bot::new(token);

    println!("Bot started. Polling for updates...");

    loop {
        match bot.get_updates() {
            Ok(updates) => {
                for update in updates {
                    if let Some(message) = update.message {
                        let chat_id = message.chat.id;

                        if let Some(text) = message.text {
                            handle_text_command(&bot, chat_id, &text);
                        } else if let Some(voice) = message.voice {
                            handle_voice_message(&bot, chat_id, &voice);
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("Telegram API returned error status") || error_msg.contains("Telegram returned ok=false") {
                    eprintln!("API Error: {}. Retrying in 10s...", error_msg);
                    thread::sleep(Duration::from_secs(10));
                } else {
                    eprintln!("Network Error: {}. Retrying in 5s...", error_msg);
                    thread::sleep(Duration::from_secs(5));
                }
            }
        }
    }
}

fn handle_text_command(bot: &Bot, chat_id: i64, text: &str) {
    let config = match config::load_config() {
        Ok(c) => c,
        Err(e) => {
            let _ = bot.send_message(chat_id, &format!("Error loading config: {}", e));
            return;
        }
    };
    let mut state = match state::load_state() {
        Ok(s) => s,
        Err(e) => {
            let _ = bot.send_message(chat_id, &format!("Error loading state: {}", e));
            return;
        }
    };

    let response = match text.trim() {
        "/health" => {
            match health::generate_health_report(&config) {
                Ok(report) => report,
                Err(e) => format!("Error generating health report: {}", e),
            }
        }
        "/brief" => {
            match brief::generate_brief(&config, &mut state) {
                Ok(b) => b,
                Err(e) => format!("Error generating brief: {}", e),
            }
        }
        "/status" => {
            "Phase 3 active. Commands: /health /brief /status /help".to_string()
        }
        "/help" => {
            "Available commands:
/health - Daily health state
/brief - Morning brief synthesis
/status - System status
/help - Show this help message".to_string()
        }
        _ if text.starts_with('/') => {
            "Unknown command. Try /help".to_string()
        }
        _ => return, // Ignore non-commands
    };

    if let Err(e) = bot.send_message(chat_id, &response) {
        eprintln!("Error sending message: {}", e);
    }
}

fn handle_voice_message(bot: &Bot, chat_id: i64, voice: &Voice) {
    let msg = "🎙 Voice received. Audio coaching coming soon — send /help for available commands.";
    if let Err(e) = bot.send_message(chat_id, msg) {
        eprintln!("Error sending voice acknowledgment: {}", e);
    }

    if let Err(e) = bot.download_voice(&voice.file_id) {
        eprintln!("Error downloading voice file {}: {}", voice.file_id, e);
    }
}

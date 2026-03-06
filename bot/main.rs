use anyhow::{Context, Result};
use phron::clients::audio::AudioProcessor;
use phron::clients::llm::LlmClient;
use phron::clients::whisper::WhisperClient;
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
    document: Option<Document>,
}

#[derive(Debug, Deserialize)]
struct Document {
    file_id: String,
    file_name: Option<String>,
    mime_type: Option<String>,
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

    fn download_file(&self, file_id: &str, dest_path: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/getFile", self.token);
        let body: TgResponse<File> = self.client.get(&url)
            .query(&[("file_id", file_id)])
            .send().context("getFile request failed")?
            .json().context("getFile parse failed")?;

        if !body.ok || body.result.is_none() {
            anyhow::bail!("getFile failed: {}", body.description.unwrap_or_default());
        }
        let file_path = body.result.unwrap().file_path
            .context("No file_path in getFile response")?;

        let download_url = format!("https://api.telegram.org/file/bot{}/{}", self.token, file_path);
        let mut resp = self.client.get(&download_url).send()
            .context("File download request failed")?;
        let mut file = fs::File::create(dest_path)
            .context("Failed to create destination file")?;
        resp.copy_to(&mut file).context("Failed to write file")?;
        Ok(())
    }

    fn download_voice(&self, file_id: &str) -> Result<()> {
        let dest = format!("/tmp/comes-audio-{}.ogg", file_id);
        self.download_file(file_id, &dest)
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
                        } else if let Some(doc) = message.document {
                            handle_document(&bot, chat_id, &doc);
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
    let ack = "🎙 Received. Analysing your voice...";
    if let Err(e) = bot.send_message(chat_id, ack) {
        eprintln!("Error sending voice acknowledgment: {}", e);
        return;
    }

    let result = process_voice(bot, voice);
    let reply = match result {
        Ok(critique) => critique,
        Err(e) => {
            eprintln!("Voice processing error: {}", e);
            format!("⚠️ Could not process audio: {}. Please try again.", e)
        }
    };

    if let Err(e) = bot.send_message(chat_id, &reply) {
        eprintln!("Error sending voice critique: {}", e);
    }
}

fn process_voice(bot: &Bot, voice: &Voice) -> Result<String> {
    let file_id = &voice.file_id;
    let ogg_path = format!("/tmp/comes-audio-{}.ogg", file_id);
    let wav_path = format!("/tmp/comes-audio-{}.wav", file_id);

    // 1. Download OGG from Telegram
    bot.download_voice(file_id).context("Download failed")?;

    // 2. Convert OGG → WAV via ffmpeg
    AudioProcessor::convert_to_wav(&ogg_path, &wav_path)
        .context("ffmpeg conversion failed")?;

    // 3. Transcribe via Whisper API
    let transcript = WhisperClient::new()
        .context("Whisper client init failed")?
        .transcribe(&wav_path)
        .context("Whisper transcription failed")?;

    // 4. Audio feature analysis via librosa (best-effort — don't fail if missing)
    let features = AudioProcessor::analyse(&wav_path).ok();

    // 5. Build critique prompt
    let audio_context = match &features {
        Some(f) => format!(
            "Audio metrics: {:.0}s duration, ~{} WPM, {} pauses ({:.0}% pause ratio), pitch {:.0}Hz mean ({}variation).",
            f.duration_seconds, f.wpm_estimate, f.pause_count,
            f.pause_ratio * 100.0, f.pitch_mean_hz, f.pitch_variation
        ),
        None => "Audio metrics unavailable — critique based on transcript only.".to_string(),
    };

    let prompt = format!(
        r#"You are an executive communication coach. Analyse this spoken answer and provide a structured critique.

Transcript:
{transcript}

{audio_context}

Provide a concise critique covering exactly these 5 dimensions:

1. **Filler words & pace** — count fillers (um, uh, like, you know), comment on WPM if available
2. **Structure & clarity** — is the answer-first (BLUF)? Is the logic MECE? Clear to a non-technical exec?
3. **Executive presence** — confidence, authority, concision under pressure
4. **Accent & pronunciation** — clarity for international audiences, any patterns to watch
5. **Voice texture** — resonance, warmth, variation (from audio metrics if available)

End with: **One thing to fix next time:** [single most impactful improvement]

Keep the whole critique under 300 words."#
    );

    // 6. Generate critique via LLM
    let critique = LlmClient::new()
        .context("LLM client init failed")?
        .generate("anthropic/claude-sonnet-4-5", &prompt, None)
        .context("LLM critique failed")?;

    // 7. Cleanup temp files
    AudioProcessor::cleanup(&[&ogg_path, &wav_path]);

    Ok(critique)
}

fn handle_document(bot: &Bot, chat_id: i64, doc: &Document) {
    let mime = doc.mime_type.as_deref().unwrap_or("");
    let name = doc.file_name.as_deref().unwrap_or("");

    if mime != "application/pdf" && !name.ends_with(".pdf") {
        let _ = bot.send_message(chat_id, "📄 Send me a PDF deck and I'll critique it. (Only PDF supported.)");
        return;
    }

    let ack = "📊 Received your deck. Reviewing...";
    if let Err(e) = bot.send_message(chat_id, ack) {
        eprintln!("Error sending deck acknowledgment: {}", e);
        return;
    }

    let result = process_document(bot, doc);
    let reply = match result {
        Ok(critique) => critique,
        Err(e) => {
            eprintln!("Deck review error: {}", e);
            format!("⚠️ Could not review deck: {}. Please try again.", e)
        }
    };

    if let Err(e) = bot.send_message(chat_id, &reply) {
        eprintln!("Error sending deck critique: {}", e);
    }
}

fn process_document(bot: &Bot, doc: &Document) -> Result<String> {
    let file_id = &doc.file_id;
    let pdf_path = format!("/tmp/comes-deck-{}.pdf", file_id);
    let txt_path = format!("/tmp/comes-deck-{}.txt", file_id);

    // 1. Download PDF from Telegram
    bot.download_file(file_id, &pdf_path).context("PDF download failed")?;

    // 2. Extract text via pdftotext
    let status = std::process::Command::new("pdftotext")
        .arg(&pdf_path)
        .arg(&txt_path)
        .status()
        .context("Failed to execute pdftotext — install poppler-utils")?;

    if !status.success() {
        anyhow::bail!("pdftotext failed with status: {}", status);
    }

    let text = std::fs::read_to_string(&txt_path)
        .context("Failed to read extracted text")?;

    if text.trim().is_empty() {
        let _ = std::fs::remove_file(&pdf_path);
        let _ = std::fs::remove_file(&txt_path);
        anyhow::bail!("No text could be extracted — deck may be image-only");
    }

    // Truncate to ~12K chars to stay within model context
    let truncated: String = text.chars().take(12_000).collect();
    let was_truncated = text.len() > truncated.len();
    let truncation_note = if was_truncated {
        "\n\n[Note: deck was truncated to first ~12K characters for analysis]"
    } else {
        ""
    };

    let prompt = format!(
        r#"You are an executive communication coach reviewing a slide deck. Critique the following extracted text from a presentation.

Deck content:
{truncated}{truncation_note}

Provide a structured critique covering exactly these 5 dimensions:

1. **Structure & flow** — Is there a clear narrative arc? Does it follow the Pyramid Principle (conclusion first, then support)? Is the logical flow easy to follow?
2. **Executive clarity** — Would a non-technical C-suite exec understand this in 90 seconds? Is the key message obvious on each slide?
3. **MECE completeness** — Are sections mutually exclusive and collectively exhaustive? Any gaps or overlaps?
4. **Evidence & substance** — Are claims backed by data or examples? Any assertions that need support?
5. **Action orientation** — Is there a clear ask or call to action? Does the deck drive a decision?

End with: **One thing to fix first:** [single highest-impact improvement]

Keep the whole critique under 350 words."#
    );

    // 3. Generate critique via LLM
    let critique = phron::clients::llm::LlmClient::new()
        .context("LLM client init failed")?
        .generate("anthropic/claude-sonnet-4-5", &prompt, None)
        .context("LLM deck critique failed")?;

    // 4. Cleanup temp files
    let _ = std::fs::remove_file(&pdf_path);
    let _ = std::fs::remove_file(&txt_path);

    Ok(critique)
}

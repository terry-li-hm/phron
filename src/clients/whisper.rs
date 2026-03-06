use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use reqwest::blocking::multipart;

#[allow(dead_code)]
pub struct WhisperClient {
    client: Client,
    api_key: String,
}

#[allow(dead_code)]
impl WhisperClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .context("OPENAI_API_KEY not set")?;
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub fn transcribe(&self, audio_path: &str) -> Result<String> {
        let file_bytes = std::fs::read(audio_path)
            .context(format!("Failed to read audio file at: {}", audio_path))?;
        
        let form = multipart::Form::new()
            .part("file", multipart::Part::bytes(file_bytes)
                .file_name("audio.wav")
                .mime_str("audio/wav")?)
            .text("model", "whisper-1");

        let resp = self.client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .context("Failed to send request to Whisper API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            bail!("Whisper API returned {}: {}", status, text);
        }

        let data: serde_json::Value = resp.json()
            .context("Failed to parse Whisper API response")?;
        
        let text = data["text"].as_str()
            .context("Missing text in Whisper response")?;

        Ok(text.to_string())
    }
}

use anyhow::{bail, Context, Result};
use reqwest::blocking::multipart;
use reqwest::blocking::Client;

#[allow(dead_code)]
pub struct WhisperClient {
    client: Client,
    api_key: String,
}

#[allow(dead_code)]
impl WhisperClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub fn transcribe(&self, audio_path: &str) -> Result<String> {
        let file_bytes = std::fs::read(audio_path)
            .context(format!("Failed to read audio file at: {}", audio_path))?;

        let form = multipart::Form::new()
            .part(
                "file",
                multipart::Part::bytes(file_bytes)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")?,
            )
            .text("model", "whisper-1");

        let resp = self
            .client
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

        let data: serde_json::Value = resp
            .json()
            .context("Failed to parse Whisper API response")?;

        extract_transcription_text(&data)
    }
}

fn extract_transcription_text(data: &serde_json::Value) -> Result<String> {
    data["text"]
        .as_str()
        .context("Missing text in Whisper response")
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_text() {
        let data = json!({"text": "hello world"});
        assert_eq!(extract_transcription_text(&data).unwrap(), "hello world");
    }

    #[test]
    fn extracts_empty_text() {
        let data = json!({"text": ""});
        assert_eq!(extract_transcription_text(&data).unwrap(), "");
    }

    #[test]
    fn missing_text_errors() {
        let err = extract_transcription_text(&json!({})).unwrap_err();
        assert!(err.to_string().contains("Missing text in Whisper response"));
    }

    #[test]
    fn null_text_errors() {
        assert!(extract_transcription_text(&json!({"text": null})).is_err());
    }

    #[test]
    fn numeric_text_errors() {
        assert!(extract_transcription_text(&json!({"text": 1})).is_err());
    }

    #[test]
    fn extra_fields_are_ignored() {
        let data = json!({"text": "ok", "language": "en"});
        assert_eq!(extract_transcription_text(&data).unwrap(), "ok");
    }
}

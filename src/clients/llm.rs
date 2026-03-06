use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde_json::json;

pub struct LlmClient {
    client: Client,
    api_key: String,
}

impl LlmClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY not set")?;
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub fn generate(&self, model: &str, prompt: &str, system: Option<&str>) -> Result<String> {
        let mut body = json!({
            "model": model,
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        if let Some(sys) = system {
            body.as_object_mut()
                .unwrap()
                .insert("system".to_string(), json!(sys));
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .context("Failed to reach Anthropic API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            bail!("Anthropic API returned {}: {}", status, text);
        }

        let data: serde_json::Value = resp.json()?;
        let content = data["content"][0]["text"]
            .as_str()
            .context("Missing content text in response")?;

        Ok(content.to_string())
    }
}

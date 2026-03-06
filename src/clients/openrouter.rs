use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde_json::json;

pub struct OpenRouterClient {
    client: Client,
    api_key: String,
}

impl OpenRouterClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY").context("OPENROUTER_API_KEY not set")?;
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub fn generate(&self, model: &str, prompt: &str) -> Result<String> {
        let body = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let resp = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .context("Failed to reach OpenRouter API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            bail!("OpenRouter API returned {}: {}", status, text);
        }

        let data: serde_json::Value = resp.json()?;
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .context("Missing content in OpenRouter response")?;

        Ok(content.to_string())
    }
}

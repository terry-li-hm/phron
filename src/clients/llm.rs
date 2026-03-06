use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde_json::json;

/// LLM client that routes through OpenRouter (OpenAI-compatible API).
/// OpenRouter supports Anthropic models via OPENROUTER_API_KEY, avoiding
/// the need for separate Anthropic API credits distinct from the Max plan.
pub struct LlmClient {
    client: Client,
    api_key: String,
}

impl LlmClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .context("OPENROUTER_API_KEY not set")?;
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub fn generate(&self, model: &str, prompt: &str, system: Option<&str>) -> Result<String> {
        let mut messages = vec![json!({"role": "user", "content": prompt})];
        if let Some(sys) = system {
            messages.insert(0, json!({"role": "system", "content": sys}));
        }

        let body = json!({
            "model": model,
            "max_tokens": 1024,
            "messages": messages
        });

        let resp = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/terry-li-hm/phron")
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

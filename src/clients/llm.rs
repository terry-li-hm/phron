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
        let api_key = std::env::var("OPENROUTER_API_KEY").context("OPENROUTER_API_KEY not set")?;
        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub fn generate(&self, model: &str, prompt: &str, system: Option<&str>) -> Result<String> {
        let messages = build_messages(prompt, system);

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
        super::extract_chat_content(&data)
    }
}

fn build_messages(prompt: &str, system: Option<&str>) -> Vec<serde_json::Value> {
    let mut messages = vec![json!({"role": "user", "content": prompt})];
    if let Some(sys) = system {
        messages.insert(0, json!({"role": "system", "content": sys}));
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_only_when_no_system() {
        let messages = build_messages("hi", None);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "hi");
    }

    #[test]
    fn system_is_prepended() {
        let messages = build_messages("hi", Some("you are a coach"));
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "you are a coach");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "hi");
    }

    #[test]
    fn empty_prompt_is_preserved() {
        let messages = build_messages("", Some(""));
        assert_eq!(messages[0]["content"], "");
        assert_eq!(messages[1]["content"], "");
    }
}

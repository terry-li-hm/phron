pub mod audio;
pub mod llm;
pub mod openrouter;
pub mod oura;
pub mod telegram;
pub mod vault;
pub mod whisper;

use anyhow::{Context, Result};

pub(crate) fn extract_chat_content(data: &serde_json::Value) -> Result<String> {
    data["choices"][0]["message"]["content"]
        .as_str()
        .context("Missing content in OpenRouter response")
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_string_content() {
        let data = json!({"choices": [{"message": {"content": "hello"}}]});
        assert_eq!(extract_chat_content(&data).unwrap(), "hello");
    }

    #[test]
    fn extracts_empty_string_content() {
        let data = json!({"choices": [{"message": {"content": ""}}]});
        assert_eq!(extract_chat_content(&data).unwrap(), "");
    }

    #[test]
    fn missing_choices_errors() {
        let err = extract_chat_content(&json!({})).unwrap_err();
        assert!(err
            .to_string()
            .contains("Missing content in OpenRouter response"));
    }

    #[test]
    fn empty_choices_errors() {
        let data = json!({"choices": []});
        assert!(extract_chat_content(&data).is_err());
    }

    #[test]
    fn null_content_errors() {
        let data = json!({"choices": [{"message": {"content": null}}]});
        assert!(extract_chat_content(&data).is_err());
    }

    #[test]
    fn array_content_errors() {
        // Some OpenAI-compatible payloads return content as an array of parts.
        let data = json!({
            "choices": [{"message": {"content": [{"type": "text", "text": "hi"}]}}]
        });
        assert!(extract_chat_content(&data).is_err());
    }

    #[test]
    fn missing_message_errors() {
        let data = json!({"choices": [{"index": 0}]});
        assert!(extract_chat_content(&data).is_err());
    }
}

use anyhow::{Context, Result, anyhow};
use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::models::{ChatResponse, Message, TokenUsage};
use crate::schemas::{review_json_schema, tools_schema};

pub(crate) fn create_model_response(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: &[Message],
    use_tools: bool,
) -> Result<ChatResponse> {
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let mut payload = json!({
        "model": model,
        "messages": messages,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "command_review_result",
                "schema": review_json_schema(),
                "strict": true
            }
        }
    });
    if use_tools {
        payload["tools"] = tools_schema();
        payload["tool_choice"] = json!("auto");
    }

    let response = post_chat(client, &endpoint, api_key, &payload)?;
    if response.status().is_success() {
        return response
            .json::<ChatResponse>()
            .context("failed to parse chat response");
    }

    let status = response.status();
    let body = response.text().unwrap_or_default();
    if status.as_u16() == 400
        && (body.to_lowercase().contains("structured output")
            || body.to_lowercase().contains("lazy grammar"))
    {
        if let Value::Object(ref mut map) = payload {
            map.remove("response_format");
        }
        let retry = post_chat(client, &endpoint, api_key, &payload)?;
        if retry.status().is_success() {
            return retry
                .json::<ChatResponse>()
                .context("failed to parse chat response");
        }
        let retry_status = retry.status();
        let retry_body = retry.text().unwrap_or_default();
        return Err(anyhow!(
            "chat completion failed with {retry_status}: {retry_body}"
        ));
    }
    Err(anyhow!("chat completion failed with {status}: {body}"))
}

fn post_chat(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    payload: &Value,
) -> Result<reqwest::blocking::Response> {
    client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(payload)
        .send()
        .context("failed to call chat completions endpoint")
}

pub(crate) fn response_message(response: &ChatResponse) -> Result<Message> {
    response
        .choices
        .first()
        .map(|choice| choice.message.clone())
        .ok_or_else(|| anyhow!("OpenAI response did not contain choices."))
}

pub(crate) fn response_text(response: &ChatResponse) -> Result<String> {
    response
        .choices
        .iter()
        .find_map(|choice| {
            choice
                .message
                .content
                .clone()
                .filter(|text| !text.is_empty())
        })
        .ok_or_else(|| anyhow!("OpenAI response did not contain text output."))
}

pub(crate) fn response_token_usage(
    response: &ChatResponse,
    messages: &[Message],
    use_tools: bool,
) -> TokenUsage {
    if let Some(usage) = &response.usage {
        let input = usage.prompt_tokens.or(usage.input_tokens).unwrap_or(0);
        let output = usage.completion_tokens.or(usage.output_tokens).unwrap_or(0);
        return TokenUsage {
            input_tokens: input,
            output_tokens: output,
            total_tokens: usage.total_tokens.unwrap_or(input + output),
            estimated: false,
        };
    }

    let mut input_payload = json!({ "messages": messages });
    if use_tools {
        input_payload["tools"] = tools_schema();
    }
    let input_tokens = estimate_token_count(&input_payload);
    let output_tokens = estimate_token_count(&json!(response.choices.first().map(|c| &c.message)));
    TokenUsage {
        input_tokens,
        output_tokens,
        total_tokens: input_tokens + output_tokens,
        estimated: true,
    }
}

pub(crate) fn combine_token_usage(left: TokenUsage, right: TokenUsage) -> TokenUsage {
    TokenUsage {
        input_tokens: left.input_tokens + right.input_tokens,
        output_tokens: left.output_tokens + right.output_tokens,
        total_tokens: left.total_tokens + right.total_tokens,
        estimated: left.estimated || right.estimated,
    }
}

fn estimate_token_count(value: &Value) -> usize {
    let text = value.to_string();
    let mut count = 0;
    let mut in_word = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            if !in_word {
                count += 1;
                in_word = true;
            }
        } else if ch.is_whitespace() {
            in_word = false;
        } else {
            count += 1;
            in_word = false;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combines_token_usage() {
        let usage = combine_token_usage(
            TokenUsage {
                input_tokens: 1,
                output_tokens: 2,
                total_tokens: 3,
                estimated: false,
            },
            TokenUsage {
                input_tokens: 4,
                output_tokens: 5,
                total_tokens: 9,
                estimated: true,
            },
        );

        assert_eq!(usage.input_tokens, 5);
        assert_eq!(usage.output_tokens, 7);
        assert_eq!(usage.total_tokens, 12);
        assert!(usage.estimated);
    }
}

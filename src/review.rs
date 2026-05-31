use std::env;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::cli::Args;
use crate::models::{ChatResponse, Message, ReviewResult, TokenUsage};
use crate::openai::{
    combine_token_usage, create_model_response, response_message, response_text,
    response_token_usage,
};
use crate::tools::{format_tool_call, run_tool_call};

pub(crate) const DECISION_REJECT: &str = "REJECT";

const DEFAULT_MODEL: &str = "google/gemma-4-e4b";
const DEFAULT_BASE_URL: &str = "http://10.0.0.232:1234/v1";
const FAST_PROMPT_SUFFIX: &str = "\n\nFast mode: make a one-shot decision using only the command text and workspace path. Do not request or call tools.";
const REVIEW_RESPONSE_RETRY_PROMPT: &str = "Return only a valid JSON object matching the required schema. The `risks` field must always be an array of strings, even when empty. Do not include markdown, prose, or tool calls.";
const DEFAULT_PROMPT: &str = include_str!("prompt.md");

pub(crate) fn review_command(command: &str, args: &Args) -> Result<ReviewResult> {
    let client = Client::new();
    let model = args
        .model
        .clone()
        .or_else(|| env::var("COMMAND_REVIEW_OPENAI_MODEL").ok())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let base_url = args
        .base_url
        .clone()
        .or_else(|| env::var("COMMAND_REVIEW_OPENAI_BASE_URL").ok())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let api_key = env::var("COMMAND_REVIEW_OPENAI_API_KEY").unwrap_or_else(|_| "local".to_string());
    let workspace = args
        .cwd
        .clone()
        .unwrap_or(env::current_dir()?)
        .canonicalize()
        .context("failed to resolve workspace path")?;

    let mut prompt = load_prompt(args.prompt_file.as_deref())?;
    if args.fast {
        prompt.push_str(FAST_PROMPT_SUFFIX);
    }

    let mut messages = vec![
        Message::system(prompt),
        Message::user(format!(
            "Workspace: {}\n\nAnalyze this shell command:\n\n{}",
            workspace.display(),
            command
        )),
    ];

    let mut token_usage = TokenUsage::default();
    let mut reported_tool_calls = Vec::new();

    if args.fast {
        let response =
            create_model_response(&client, &base_url, &api_key, &model, &messages, false)?;
        token_usage = combine_token_usage(
            token_usage,
            response_token_usage(&response, &messages, false),
        );
        return parse_or_repair_response(
            &client,
            &base_url,
            &api_key,
            &model,
            messages,
            response,
            token_usage,
        );
    }

    loop {
        let response =
            create_model_response(&client, &base_url, &api_key, &model, &messages, true)?;
        token_usage = combine_token_usage(
            token_usage,
            response_token_usage(&response, &messages, true),
        );
        let assistant = response_message(&response)?;
        let tool_calls = assistant.tool_calls.clone().unwrap_or_default();
        messages.push(assistant);

        if tool_calls.is_empty() {
            return match parse_review_response(
                messages
                    .last()
                    .and_then(|m| m.content.as_deref())
                    .unwrap_or(""),
            ) {
                Ok(mut result) => {
                    result.tool_calls = reported_tool_calls;
                    result.token_usage = token_usage;
                    Ok(result)
                }
                Err(err) => {
                    let retry = Message::user(format!(
                        "Your previous response could not be parsed as a command review result: {err}\n\n{REVIEW_RESPONSE_RETRY_PROMPT}"
                    ));
                    messages.push(retry);
                    let repair = create_model_response(
                        &client, &base_url, &api_key, &model, &messages, false,
                    )?;
                    token_usage = combine_token_usage(
                        token_usage,
                        response_token_usage(&repair, &messages, false),
                    );
                    let mut result = parse_review_response(&response_text(&repair)?)?;
                    result.tool_calls = reported_tool_calls;
                    result.token_usage = token_usage;
                    Ok(result)
                }
            };
        }

        for tool_call in tool_calls {
            let output = run_tool_call(&tool_call, &workspace);
            let reported = format_tool_call(&tool_call, &output);
            if args.stream_tools {
                eprintln!(
                    "{}",
                    serde_json::to_string(&json!({ "tool_call": reported }))?
                );
            }
            reported_tool_calls.push(reported);
            messages.push(Message::tool(tool_call.id, tool_call.function.name, output));
        }
    }
}

fn parse_or_repair_response(
    client: &Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    mut messages: Vec<Message>,
    response: ChatResponse,
    mut token_usage: TokenUsage,
) -> Result<ReviewResult> {
    match parse_review_response(&response_text(&response)?) {
        Ok(mut result) => {
            result.token_usage = token_usage;
            Ok(result)
        }
        Err(err) => {
            messages.push(response_message(&response)?);
            messages.push(Message::user(format!(
                "Your previous response could not be parsed as a command review result: {err}\n\n{REVIEW_RESPONSE_RETRY_PROMPT}"
            )));
            let repair = create_model_response(client, base_url, api_key, model, &messages, false)?;
            token_usage =
                combine_token_usage(token_usage, response_token_usage(&repair, &messages, false));
            let mut result = parse_review_response(&response_text(&repair)?)?;
            result.token_usage = token_usage;
            Ok(result)
        }
    }
}

fn load_prompt(path: Option<&Path>) -> Result<String> {
    match path {
        Some(path) => Ok(fs::read_to_string(path)?.trim().to_string()),
        None => Ok(DEFAULT_PROMPT.trim().to_string()),
    }
}

fn parse_review_response(content: &str) -> Result<ReviewResult> {
    let payload = match serde_json::from_str::<Value>(content) {
        Ok(payload) => payload,
        Err(_) => serde_json::from_str::<Value>(&extract_json_object(content)?)?,
    };

    let decision = expect_str(&payload, "decision")?;
    let risk_level = expect_str(&payload, "risk_level")?;
    let summary = expect_str(&payload, "summary")?;
    let risks = payload
        .get("risks")
        .and_then(Value::as_array)
        .filter(|items| items.iter().all(Value::is_string))
        .ok_or_else(|| anyhow!("OpenAI response field `risks` must be a list of strings."))?
        .iter()
        .map(|item| item.as_str().unwrap().to_string())
        .collect();
    let reasoning = expect_str(&payload, "reasoning")?;

    Ok(ReviewResult {
        decision,
        risk_level,
        summary,
        risks,
        reasoning,
        tool_calls: Vec::new(),
        token_usage: TokenUsage::default(),
    })
}

fn expect_str(payload: &Value, key: &str) -> Result<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow!("OpenAI response field `{key}` must be a string."))
}

fn extract_json_object(content: &str) -> Result<String> {
    let start = content
        .find('{')
        .ok_or_else(|| anyhow!("OpenAI response did not contain a JSON object."))?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;

    for (offset, ch) in content[start..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Ok(content[start..start + offset + ch.len_utf8()].to_string());
            }
        }
    }
    Err(anyhow!("OpenAI response JSON object was incomplete."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_wrapped_in_markdown_text() {
        let result = parse_review_response(
            "Here is the review:\n```json\n{\"decision\":\"APPROVE\",\"risk_level\":\"LOW\",\"summary\":\"Reviewed command.\",\"risks\":[\"Reviewed by model.\"],\"reasoning\":\"Model-provided decision.\"}\n```",
        )
        .unwrap();

        assert_eq!(result.decision, "APPROVE");
        assert_eq!(result.risks, vec!["Reviewed by model."]);
    }

    #[test]
    fn rejects_non_string_risks() {
        let err = parse_review_response(
            "{\"decision\":\"APPROVE\",\"risk_level\":\"LOW\",\"summary\":\"Reviewed command.\",\"risks\":\"bad\",\"reasoning\":\"Model-provided decision.\"}",
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("risks"));
    }
}

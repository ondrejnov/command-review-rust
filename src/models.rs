use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub(crate) struct TokenUsage {
    pub(crate) input_tokens: usize,
    pub(crate) output_tokens: usize,
    pub(crate) total_tokens: usize,
    pub(crate) estimated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ReviewResult {
    pub(crate) decision: String,
    pub(crate) risk_level: String,
    pub(crate) summary: String,
    pub(crate) risks: Vec<String>,
    pub(crate) reasoning: String,
    #[serde(default)]
    pub(crate) tool_calls: Vec<ReportedToolCall>,
    #[serde(default)]
    pub(crate) token_usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ReportedToolCall {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) arguments: Value,
    pub(crate) output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Message {
    pub(crate) role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolCall {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) call_type: String,
    pub(crate) function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FunctionCall {
    pub(crate) name: String,
    pub(crate) arguments: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatResponse {
    pub(crate) choices: Vec<Choice>,
    pub(crate) usage: Option<ApiUsage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Choice {
    pub(crate) message: Message,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApiUsage {
    pub(crate) prompt_tokens: Option<usize>,
    pub(crate) completion_tokens: Option<usize>,
    pub(crate) total_tokens: Option<usize>,
    pub(crate) input_tokens: Option<usize>,
    pub(crate) output_tokens: Option<usize>,
}

impl Message {
    pub(crate) fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub(crate) fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    pub(crate) fn tool(tool_call_id: String, name: String, content: String) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
            name: Some(name),
        }
    }
}

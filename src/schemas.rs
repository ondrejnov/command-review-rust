use serde_json::{Value, json};

pub(crate) fn review_json_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "decision": { "type": "string", "enum": ["APPROVE", "REQUIRE_CONFIRMATION", "REJECT"] },
            "risk_level": { "type": "string", "enum": ["LOW", "MEDIUM", "HIGH", "CRITICAL"] },
            "summary": { "type": "string" },
            "risks": { "type": "array", "items": { "type": "string" } },
            "reasoning": { "type": "string" }
        },
        "required": ["decision", "risk_level", "summary", "risks", "reasoning"],
        "additionalProperties": false
    })
}

pub(crate) fn tools_schema() -> Value {
    json!([
        {
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files and directories under a workspace-relative path.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Workspace-relative directory path. Defaults to the workspace root."
                        }
                    },
                    "additionalProperties": false
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read a workspace-relative text file.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Workspace-relative file path to read."
                        }
                    },
                    "required": ["path"],
                    "additionalProperties": false
                }
            }
        }
    ])
}

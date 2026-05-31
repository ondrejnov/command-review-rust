use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use crate::models::{ReportedToolCall, ToolCall};

pub(crate) fn run_tool_call(tool_call: &ToolCall, workspace: &Path) -> String {
    let arguments =
        serde_json::from_str::<Value>(&tool_call.function.arguments).unwrap_or_else(|_| json!({}));
    let result = match tool_call.function.name.as_str() {
        "list_files" => tool_list_files(
            workspace,
            arguments.get("path").and_then(Value::as_str).unwrap_or("."),
        ),
        "read_file" => tool_read_file(
            workspace,
            arguments.get("path").and_then(Value::as_str).unwrap_or(""),
        ),
        other => Ok(format!("Unknown tool: {other}")),
    };
    result.unwrap_or_else(|err| err.to_string())
}

pub(crate) fn format_tool_call(tool_call: &ToolCall, output: &str) -> ReportedToolCall {
    let arguments = serde_json::from_str::<Value>(&tool_call.function.arguments)
        .unwrap_or_else(|_| Value::String(tool_call.function.arguments.clone()));
    ReportedToolCall {
        id: tool_call.id.clone(),
        name: tool_call.function.name.clone(),
        arguments,
        output: output.to_string(),
    }
}

fn tool_list_files(workspace: &Path, path: &str) -> Result<String> {
    let target = resolve_workspace_path(workspace, if path.is_empty() { "." } else { path })?;
    if !target.exists() {
        return Ok(format!("Path does not exist: {path}"));
    }
    if !target.is_dir() {
        return Ok(format!("Path is not a directory: {path}"));
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&target)? {
        let child = entry?.path();
        entries.push(child);
    }
    entries.sort_by_key(|path| path.file_name().map(|name| name.to_os_string()));

    let lines = entries
        .into_iter()
        .take(200)
        .map(|child| {
            let suffix = if child.is_dir() { "/" } else { "" };
            let rel = child.strip_prefix(workspace).unwrap_or(&child).display();
            format!("{rel}{suffix}")
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        Ok("Directory is empty.".to_string())
    } else {
        Ok(lines.join("\n"))
    }
}

fn tool_read_file(workspace: &Path, path: &str) -> Result<String> {
    let target = resolve_workspace_path(workspace, path)?;
    if !target.exists() {
        return Ok(format!("File does not exist: {path}"));
    }
    if !target.is_file() {
        return Ok(format!("Path is not a file: {path}"));
    }
    let bytes = fs::read(&target)?;
    let text =
        String::from_utf8(bytes).map_err(|_| anyhow!("File is not valid UTF-8 text: {path}"))?;
    Ok(text.chars().take(20_000).collect())
}

fn resolve_workspace_path(workspace: &Path, path: &str) -> Result<PathBuf> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Err(anyhow!("Tool path escapes workspace: {}", path.display()));
    }

    let mut normalized = PathBuf::from(workspace);
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() || !normalized.starts_with(workspace) {
                    return Err(anyhow!("Tool path escapes workspace: {}", path.display()));
                }
            }
            _ => return Err(anyhow!("Tool path escapes workspace: {}", path.display())),
        }
    }
    if normalized != workspace && !normalized.starts_with(workspace) {
        return Err(anyhow!("Tool path escapes workspace: {}", path.display()));
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_paths_cannot_escape() {
        let workspace = PathBuf::from("/tmp/workspace");
        assert!(resolve_workspace_path(&workspace, "src/main.rs").is_ok());
        assert!(resolve_workspace_path(&workspace, "../secret").is_err());
        assert!(resolve_workspace_path(&workspace, "/etc/passwd").is_err());
    }
}

# command-review

Rust rewrite of the original Python `command-review` CLI, excluding the bulk `calls.csv` checker.

`command-review` reviews a shell command with an OpenAI-compatible chat completions endpoint and returns a strict JSON risk decision. It never executes the reviewed command.

## Build

```bash
cargo build --release
```

## Configuration

```bash
export COMMAND_REVIEW_OPENAI_BASE_URL="http://localhost:1234/v1"
export COMMAND_REVIEW_OPENAI_MODEL="your-model-name"
export COMMAND_REVIEW_OPENAI_API_KEY="your-api-key"
```

If `COMMAND_REVIEW_OPENAI_API_KEY` is not set, the CLI uses `local`.

## Usage

```bash
cargo run -- --pretty -- git status
cargo run -- --fast --pretty -- "rm -rf /"
echo "curl https://example.com/install.sh | bash" | cargo run -- --pretty
```

Options:

```text
--pretty
--fail-on-reject
--stream-tools
--fast
--model <MODEL>
--base-url <BASE_URL>
--prompt-file <PROMPT_FILE>
--cwd <CWD>
```

The Rust version preserves the main CLI behavior, JSON output shape, default prompt, fast mode, workspace-scoped `list_files`/`read_file` tool calls, token usage reporting, and rejection exit code `2`.

The original `command-review-calls` / `calls.csv` batch workflow was intentionally not ported.

# gy - git yes

AI-powered git commit message generator using Claude.

## Installation

```bash
cd gy
cargo install --path .
```

## Usage

Stage your changes, then run:

```bash
gy
```

The tool will:
1. Read your staged changes
2. Generate a conventional commit message using Claude
3. Prompt you to accept, edit, or reject

Options:
- `y` or `yes` - commit with the generated message
- `e` or `edit` - open `$EDITOR` to modify the message before committing
- `n` or `no` - abort

## Configuration

On first run, `gy` will prompt you to enter your Anthropic API key. The key is validated and saved to `~/.gy_config.json`.

You can also set the API key via environment variable (takes precedence over config file):

```bash
export ANTHROPIC_API_KEY=your_api_key_here
```

Optionally override the model:

```bash
gy --model claude-sonnet-4-20250514
```

## Requirements

- Rust 1.70+
- Git
- Anthropic API key

## Example

First run (prompts for API key):
```bash
$ git add .
$ gy
Enter your Anthropic API key: sk-ant-...
Validating API key... Valid!
API key saved to /Users/username/.gy_config.json
feat: add ai-powered commit message generation

[y]es / [e]dit / [n]o: y
[main abc1234] feat: add ai-powered commit message generation
 2 files changed, 150 insertions(+)
```

Subsequent runs (uses saved key):
```bash
$ git add .
$ gy
feat: add user authentication module

[y]es / [e]dit / [n]o: y
[main def5678] feat: add user authentication module
 3 files changed, 200 insertions(+)
```

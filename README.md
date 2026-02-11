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

Set your Anthropic API key:

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

```bash
$ git add .
$ gy
feat: add ai-powered commit message generation

[y]es / [e]dit / [n]o: y
[main abc1234] feat: add ai-powered commit message generation
 2 files changed, 150 insertions(+)
```

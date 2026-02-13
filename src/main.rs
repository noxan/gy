use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "gy")]
#[command(about = "AI-powered git commit message generator", long_about = None)]
struct Args {
    /// Model to use for generation
    #[arg(long, default_value = "claude-haiku-4-5-20251001")]
    model: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    system: String,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<Content>,
}

#[derive(Deserialize)]
struct Content {
    text: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Deserialize)]
struct ErrorDetail {
    message: String,
}

#[derive(Serialize, Deserialize)]
struct Config {
    anthropic_api_key: String,
}

enum EditError {
    Aborted,
    Other(String),
}

fn main() {
    let args = Args::parse();

    // Get or prompt for API key
    let api_key = get_or_prompt_api_key();

    // Get staged diff
    let diff = match get_staged_diff() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if diff.trim().is_empty() {
        // No staged changes - check for unstaged changes
        match get_unstaged_diff() {
            Ok(unstaged_diff) if !unstaged_diff.trim().is_empty() => {
                eprintln!("No changes are staged. Here's what's unstaged:\n");
                match generate_commit_message(&api_key, &args.model, &unstaged_diff) {
                    Ok(summary) => {
                        println!("{}\n", summary);
                    }
                    Err(_) => {
                        // If AI generation fails, just show a simple message
                    }
                }
                eprintln!("Use 'git add' to stage changes.");
                std::process::exit(1);
            }
            _ => {
                eprintln!("Nothing staged. Use git add first.");
                std::process::exit(1);
            }
        }
    }

    // Generate commit message
    let commit_message = match generate_commit_message(&api_key, &args.model, &diff) {
        Ok(msg) => msg,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if commit_message.trim().is_empty() {
        eprintln!("Failed to generate commit message.");
        std::process::exit(1);
    }

    // Interactive inline editing
    let final_message = match edit_message_inline(&commit_message) {
        Ok(msg) => msg,
        Err(EditError::Aborted) => {
            eprintln!("Aborted.");
            std::process::exit(1);
        }
        Err(EditError::Other(e)) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    commit(&final_message);
}

fn get_config_path() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    home.join(".gy_config.json")
}

fn load_config() -> Option<Config> {
    let config_path = get_config_path();
    if !config_path.exists() {
        return None;
    }
    let contents = fs::read_to_string(config_path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn save_config(config: &Config) -> Result<(), String> {
    let config_path = get_config_path();
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(config_path, json)
        .map_err(|e| format!("Failed to write config: {}", e))?;
    Ok(())
}

fn validate_api_key(api_key: &str) -> Result<(), String> {
    let request = AnthropicRequest {
        model: "claude-haiku-4-5-20251001".to_string(),
        max_tokens: 10,
        messages: vec![Message {
            role: "user".to_string(),
            content: "test".to_string(),
        }],
        system: "Reply with ok".to_string(),
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().unwrap_or_default();

        if let Ok(error_resp) = serde_json::from_str::<ErrorResponse>(&error_text) {
            return Err(error_resp.error.message);
        }

        return Err(format!("API error ({})", status));
    }

    Ok(())
}

fn get_or_prompt_api_key() -> String {
    // First check environment variable
    if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            return key;
        }
    }

    // Then check config file
    if let Some(config) = load_config() {
        if !config.anthropic_api_key.is_empty() {
            return config.anthropic_api_key;
        }
    }

    // Prompt user for API key
    loop {
        print!("Enter your Anthropic API key: ");
        io::stdout().flush().unwrap();

        let mut api_key = String::new();
        io::stdin().read_line(&mut api_key).unwrap();
        let api_key = api_key.trim().to_string();

        if api_key.is_empty() {
            eprintln!("API key cannot be empty. Please try again.");
            continue;
        }

        print!("Validating API key...");
        io::stdout().flush().unwrap();

        match validate_api_key(&api_key) {
            Ok(_) => {
                println!(" Valid!");
                let config = Config {
                    anthropic_api_key: api_key.clone(),
                };
                if let Err(e) = save_config(&config) {
                    eprintln!("Warning: Failed to save config: {}", e);
                } else {
                    println!("API key saved to {}", get_config_path().display());
                }
                return api_key;
            }
            Err(e) => {
                println!(" Invalid!");
                eprintln!("Error: {}", e);
                eprintln!("Please try again with a valid API key.");
            }
        }
    }
}

fn get_staged_diff() -> Result<String, String> {
    let output = Command::new("git")
        .args(["diff", "--staged"])
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn get_unstaged_diff() -> Result<String, String> {
    let output = Command::new("git")
        .args(["diff"])
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn generate_commit_message(api_key: &str, model: &str, diff: &str) -> Result<String, String> {
    let system_prompt = "You are a git commit message generator. Given a git diff, produce a single conventional commit message (type: description). Use lowercase. Be concise. Output ONLY the commit message, nothing else. If the diff includes multiple logical changes, use the most significant one for the type. Types: feat, fix, refactor, docs, style, test, chore, perf, ci, build.";

    let request = AnthropicRequest {
        model: model.to_string(),
        max_tokens: 256,
        messages: vec![Message {
            role: "user".to_string(),
            content: diff.to_string(),
        }],
        system: system_prompt.to_string(),
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| format!("API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().unwrap_or_default();

        // Try to parse as error response
        if let Ok(error_resp) = serde_json::from_str::<ErrorResponse>(&error_text) {
            return Err(format!("API error: {}", error_resp.error.message));
        }

        return Err(format!("API error ({}): {}", status, error_text));
    }

    let api_response: AnthropicResponse = response
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    if api_response.content.is_empty() {
        return Err("Empty response from API".to_string());
    }

    Ok(api_response.content[0].text.trim().to_string())
}

fn edit_message_inline(message: &str) -> Result<String, EditError> {
    let mut rl = DefaultEditor::new().map_err(|e| EditError::Other(e.to_string()))?;

    eprintln!("Enter to commit • Esc to abort");

    match rl.readline_with_initial("", (message, "")) {
        Ok(line) => {
            let edited = line.trim();
            if edited.is_empty() {
                return Err(EditError::Other(
                    "Commit message cannot be empty".to_string(),
                ));
            }
            Ok(edited.to_string())
        }
        Err(ReadlineError::Interrupted) => Err(EditError::Aborted),
        Err(ReadlineError::Eof) => Err(EditError::Aborted),
        Err(e) => Err(EditError::Other(e.to_string())),
    }
}

fn commit(message: &str) {
    let status = Command::new("git")
        .args(["commit", "-m", message])
        .status()
        .expect("Failed to run git commit");

    if !status.success() {
        eprintln!("git commit failed");
        std::process::exit(1);
    }
}

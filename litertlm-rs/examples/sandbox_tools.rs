//! Minimal example: give the model two tools (list_files, read_file)
//! confined to a sandbox directory, so it can answer questions about files
//! there without having any other filesystem access.
//!
//! Usage: sandbox_tools <path-to-model.litertlm> <path-to-sandbox-dir>
//!
//! Schema verified against LiteRT-LM's own docs
//!
//! A tool-call response looks like:
//!   {
//!     "tool_calls": [{
//!       "type": "function",
//!       "function": {
//!         "name": "...",
//!         "arguments": {...}
//!       }
//!     }]
//!   }
//!
//! The reply sent back for each tool call is:
//!   {
//!     "role": "tool",
//!     "content":[
//!       "type": "tool_response",
//!       "name": "...",
//!       "response": <raw JSON object the tool produced>
//!     ]
//!   }
//!
//! Not every model supports tool calling -- it depends on that specific
//! model's own built-in chat template.
//!
//! Deliberately minimal otherwise -- no command execution, no confirmation
//! prompts, no OS-level sandboxing (Landlock/bubblewrap). The one safety
//! check that matters here is `safe_path`, which keeps `read_file` from
//! ever leaving the sandbox directory regardless of what path the model
//! asks for.

use litertlm_rs::{
    extract_text, set_min_log_level, Conversation, ConversationConfig, Engine, EngineSettings,
    LogSeverity,
};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let model_path = args
        .next()
        .expect("usage: sandbox_tools <model> <sandbox_dir>");
    let sandbox_dir = args
        .next()
        .expect("usage: sandbox_tools <model> <sandbox_dir>");
    let sandbox_root = fs::canonicalize(&sandbox_dir)?;

    // Un-silence this (or set to LogSeverity::Error) if you need to see the
    // underlying C++ diagnostic for a `send_message` failure -- the plain
    // "returned NULL" Rust error alone doesn't show the real reason.
    set_min_log_level(LogSeverity::Silent);

    let settings = EngineSettings::new(&model_path, "cpu", None, None)?;
    let engine = Engine::new(&settings)?;

    let tools = serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List files in the sandbox directory.",
                "parameters": { "type": "object", "properties": {} }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file in the sandbox directory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "relative path within the sandbox" }
                    },
                    "required": ["path"]
                }
            }
        }
    ])
    .to_string();

    let system_prompt = r###"
You are an expert assistant specialized in interacting with a file system using provided tools (`list_files` and `read_file`).

**Your Core Directives:**

1.  **Tool Usage:** You must use the provided tools (`list_files` and `read_file`) to answer user requests. Do not attempt to answer questions about files if the tool is not necessary.`list_file`
2.  **Context Management:** Maintain context regarding the current state of the file system (i.e., the list of files available). If a user asks about files, you should reference the most recent file list you have access to.
3.  **Error Handling:** If a tool fails or returns an unexpected format, inform the user clearly about the failure and ask for clarification on the expected format.
4.  **Conversation Flow:** Be proactive. If a user asks a question that requires file access, immediately use the appropriate tool. If a user asks about the file system, use `list_files` first.

**Initial State:**
Start by acknowledging your role and readiness to use the tools. Do not generate any file lists or content until a user explicitly asks you to perform an action (e.g., "List the files," or "Read Chapter_19.md").
"###.to_string();

    let system_json = serde_json::json!({ "role": "system", "content": system_prompt }).to_string();

    let mut conversation = {
        let mut cfg = ConversationConfig::new()?;
        cfg.set_tools(&tools)?;
        cfg.set_system_message(&system_json)?;
        engine.create_conversation_with_config(&cfg)?
    };
    let mut tools_enabled = true;

    eprintln!("Sandbox: {}", sandbox_root.display());
    eprintln!("Ready. Ask about the files in that directory.\n");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        let prompt = line.trim();
        if prompt.is_empty() {
            continue;
        }

        let message = serde_json::json!({ "role": "user", "content": prompt }).to_string();

        match run_turn(&conversation, &sandbox_root, tools_enabled, message.clone()) {
            Ok(()) => {}
            // Some models' chat templates don't support tool calling at
            // all -- ANY send_message fails once tools are attached, even
            // for a message with no tool call involved. If that happens
            // on this session's very first attempt, fall back to a plain
            // conversation and retry the same message once.
            Err(e) if tools_enabled => {
                eprintln!(
                    "Note: that failed ({e}). This model's chat template likely doesn't \
                     support tool calling -- falling back to a plain conversation for the \
                     rest of this session. File questions won't work from here on."
                );
                conversation = engine.create_conversation()?;
                tools_enabled = false;
                if let Err(e) = run_turn(&conversation, &sandbox_root, tools_enabled, message) {
                    eprintln!("error: {e}");
                }
            }
            Err(e) => eprintln!("error: {e}"),
        }
        println!();
    }

    Ok(())
}

/// Runs one user turn to completion, following any tool-call round trips
/// until the model gives a final natural-language answer. Loops rather
/// than handling just one round, since the model may call a tool, see the
/// result, and then decide to call another tool before answering.
fn run_turn(
    conversation: &Conversation,
    sandbox_root: &Path,
    tools_enabled: bool,
    mut message: String,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let response = conversation.send_message(&message)?;
        let parsed: serde_json::Value = serde_json::from_str(&response)?;

        let tool_calls = if tools_enabled {
            parsed.get("tool_calls").and_then(|v| v.as_array())
        } else {
            None
        };

        match tool_calls {
            Some(calls) if !calls.is_empty() => {
                let tool_messages: Vec<serde_json::Value> = calls
                    .iter()
                    .map(|call| {
                        let function = &call["function"];
                        let name = function["name"].as_str().unwrap_or("");
                        let result = handle_tool_call(sandbox_root, name, &function["arguments"]);
                        // `content` is the tool's raw JSON result object,
                        // unwrapped -- see the module doc comment.
                        serde_json::json!({ "role": "tool", "content": result })
                    })
                    .collect();
                message = serde_json::Value::Array(tool_messages).to_string();
                // Loop again: send the tool results back and see whether
                // the model answers or calls another tool.
            }
            _ => {
                println!("{}", extract_text(&response));
                return Ok(());
            }
        }
    }
}

/// Executes one tool call, returning a JSON object (never a bare string --
/// see the module doc comment for why that distinction matters). Every
/// branch that touches the filesystem goes through `safe_path` first --
/// nothing here trusts the path the model supplied until that check has
/// run.
fn handle_tool_call(
    sandbox_root: &Path,
    name: &str,
    args: &serde_json::Value,
) -> serde_json::Value {
    match name {
        "list_files" => match fs::read_dir(sandbox_root) {
            Ok(entries) => {
                let files: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect();
                let resp = serde_json::json!([{
                      "type": "tool_response",
                      "name": "list_files",
                      "response": { "files": files }
                }]);
                resp
            }
            Err(e) => serde_json::json!({ "error": e.to_string() }),
        },
        "read_file" => {
            let Some(rel) = args.get("path").and_then(|p| p.as_str()) else {
                return serde_json::json!({ "error": "missing path" });
            };
            match safe_path(sandbox_root, rel) {
                Some(full) => match fs::read_to_string(full) {
                    Ok(content) => {
                        serde_json::json!([{
                              "type": "tool_response",
                              "name": "read_file",
                              "response": { "content": content }
                        }])
                    }
                    Err(e) => serde_json::json!({ "error": e.to_string() }),
                },
                None => serde_json::json!({ "error": "path escapes sandbox" }),
            }
        }
        _ => serde_json::json!({ "error": format!("unknown tool {name}") }),
    }
}

/// The core sandbox check: join the untrusted relative path onto the
/// sandbox root, canonicalize (resolves ".." AND symlinks), then verify
/// the result still lives inside the sandbox root before returning it.
/// This is what stops both plain "../../etc/passwd" traversal and a
/// symlink inside the sandbox that points somewhere outside it.
fn safe_path(sandbox_root: &Path, relative: &str) -> Option<PathBuf> {
    let candidate = sandbox_root.join(relative);
    let canonical = fs::canonicalize(candidate).ok()?;
    if canonical.starts_with(sandbox_root) {
        Some(canonical)
    } else {
        None
    }
}

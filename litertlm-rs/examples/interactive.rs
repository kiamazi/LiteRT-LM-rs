//! Interactive chat: reads prompts from stdin in a loop, streaming each
//! response back before waiting for the next prompt. All turns share the
//! same `Conversation`, so the model keeps context across the session.
//! Exit any time with Ctrl+C (or Ctrl+D / an empty line to quit cleanly).

use litertlm_rs::{extract_text, set_min_log_level, Engine, EngineSettings, LogSeverity};
use std::io::{self, BufRead, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = std::env::args()
        .nth(1)
        .expect("usage: interactive <path-to-model>");
    if !Path::new(&model_path).exists() {
        panic!("model path is not valid");
    }
    if let Some(stem) = Path::new(&model_path).file_stem() {
        println!("Molde: {}", stem.display());
    }

    let backend = std::env::args()
        .nth(2)
        .unwrap_or("cpu".into());

    println!("backend: {backend}");

    set_min_log_level(LogSeverity::Silent);

    eprintln!("Loading model...");
    let settings = EngineSettings::new(&model_path, &backend, None, None)?;
    let engine = Engine::new(&settings)?;
    let conversation = engine.create_conversation()?;
    eprintln!("Ready. Type a message and press Enter (Ctrl+C or Ctrl+D to quit).\n");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        // read_line returns Ok(0) at EOF (e.g. Ctrl+D), so treat that the
        // same as the user asking to quit.
        let bytes_read = stdin.lock().read_line(&mut line)?;
        if bytes_read == 0 {
            println!("\nGoodbye!");
            break;
        }

        let prompt = line.trim();
        if prompt.is_empty() {
            continue;
        }

        // The message JSON schema your build expects -- see the note in
        // basic.rs / streaming.rs. Escaping the prompt through
        // serde_json::json! avoids breaking on quotes or newlines the user
        // types.
        let message_json = serde_json::json!({
            "role": "user",
            "content": prompt,
        })
        .to_string();

        let mut printed_anything = false;
        let result = conversation.send_message_stream(&message_json, |chunk| {
            if let Some(raw) = chunk.text() {
                let text = extract_text(&raw);
                if !text.is_empty() {
                    print!("{text}");
                    io::stdout().flush().ok();
                    printed_anything = true;
                }
            }
        });

        if printed_anything {
            println!();
        }
        if let Err(e) = result {
            eprintln!("[error] {e}");
        }
        println!();
    }

    Ok(())
}

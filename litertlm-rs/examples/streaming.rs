use litertlm_rs::{extract_text, set_min_log_level, Engine, EngineSettings, LogSeverity};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = std::env::args()
        .nth(1)
        .expect("usage: streaming <path-to-model>");

    let backend = std::env::args()
        .nth(2)
        .unwrap_or("cpu".into());

    // Quiet the underlying C++ library's own stderr logging (INFO/WARNING
    // lines about accelerator registration, model loading, etc.) so only
    // your own program's output shows. Use LogSeverity::Error instead if
    // you still want to see real errors, just not the routine INFO/WARNING
    // noise.
    set_min_log_level(LogSeverity::Silent);

    let settings = EngineSettings::new(&model_path, &backend, None, None)?;
    let engine = Engine::new(&settings)?;
    let conversation = engine.create_conversation()?;

    conversation.send_message_stream(r#"{"role":"user","content":"tell me a joke"}"#, |chunk| {
        if let Some(raw) = chunk.text() {
            let text = extract_text(&raw);
            if !text.is_empty() {
                print!("{text}");
                std::io::stdout().flush().ok();
            } else {
                eprintln!("[debug] chunk didn't parse as expected text content: {raw}");
            }
        }
        if let Some(err) = chunk.error() {
            eprintln!("\n[stream error] {err}");
        }
    })?;
    println!();

    Ok(())
}

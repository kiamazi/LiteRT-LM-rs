use litertlm_rs::{extract_text, set_min_log_level, Engine, EngineSettings, LogSeverity};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = std::env::args()
        .nth(1)
        .expect("usage: basic <path-to-model>");

    let backend = std::env::args()
        .nth(2)
        .unwrap_or("cpu".into());

    set_min_log_level(LogSeverity::Silent);

    let mut settings = EngineSettings::new(&model_path, &backend, None, None)?;
    settings.set_num_threads(4);

    let engine = Engine::new(&settings)?;
    let conversation = engine.create_conversation()?;

    // Adjust this JSON to whatever message schema your build of LiteRT LM
    // expects (see the upstream docs / examples for the exact shape).
    let response = conversation.send_message(r#"{"role":"user","content":"Hello!"}"#)?;
    println!("{}", extract_text(&response));

    Ok(())
}

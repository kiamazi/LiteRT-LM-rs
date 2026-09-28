//! it's Interactive chat(interactive.rs) with `ConversationConfig` configs:
//! SamplerParams(topP, topK, temperature), SessionConfig, ThinkingConfig,
//! set_max_output.
//! reads prompts from stdin in a loop, streaming each response(thinking+text)
//! back before waiting for the next prompt. All turns share the same `Conversation`,
//! so the model keeps context across the session.
//! Exit any time with Ctrl+C (or Ctrl+D / an empty line to quit cleanly).

use litertlm_rs::{
    extract_text, set_min_log_level, ConversationConfig, Engine, EngineSettings, LogSeverity,
    SamplerParams, SamplerType, SessionConfig, ThinkingConfig,
};
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

    let backend = std::env::args().nth(2).unwrap_or("cpu".into());

    println!("backend: {backend}");

    set_min_log_level(LogSeverity::Silent);
    // set_min_log_level(LogSeverity::Silent);    // No output at all
    // set_min_log_level(LogSeverity::Error);     // Only errors
    // set_min_log_level(LogSeverity::Warning);   // Errors + warnings
    // set_min_log_level(LogSeverity::Info);      // Errors + warnings + info
    // set_min_log_level(LogSeverity::Debug);     // All except verbose
    // set_min_log_level(LogSeverity::Verbose);   // Everything

    eprintln!("Loading model...");

    let settings = EngineSettings::new(&model_path, &backend, None, None)?;
    // .set_num_threads(4)
    // .set_max_num_tokens(4096)
    // .set_cache_dir("/tmp/litertlm_cache")
    // .set_parallel_file_section_loading(true)
    //   more...
    // .set_activation_data_type(ActivationDataType::Float32);  // Default
    // .set_activation_data_type(ActivationDataType::Float16);  // Less memory
    // .set_activation_data_type(ActivationDataType::Int16);    // Quantized
    // .set_activation_data_type(ActivationDataType::Int8);     // Most quantized

    //------
    let mut sampler_params = SamplerParams::new(SamplerType::TopP)?;
    sampler_params
        .set_top_k(200)
        .set_top_p(0.9)
        .set_temperature(0.9);
    //  .set_seed(i32)

    //------
    let mut session_config = SessionConfig::new()?;
    session_config
        .set_sampler_params(&sampler_params)
        .set_max_output_tokens(8192);
    //  .set_apply_prompt_template(bool)
    //  .set_lora_path(&str)
    //  .set_audio_lora_path(&str)

    //-------
    let mut thinking_config = ThinkingConfig::new()?;
    thinking_config
        .set_enable_thinking(true)
        .set_thinking_token_budget(-1); //unlimited

    //-------
    let mut cfg = ConversationConfig::new()?;
    cfg.set_session_config(&session_config);
    cfg.set_thinking_config(&thinking_config);
    // cfg.set_enable_constrained_decoding(bool);
    // cfg.set_constraint_provider(Some(ConstraintProviderType::LlGuidance));
    // cfg.set_extra_context(&str);
    // cfg.set_filter_channel_content_from_kv_cache(bool);
    // cfg.set_messages(&str);
    // cfg.set_prompt_template(&str); // (e.g. Jinja template)
    // cfg.set_stream_tool_calls(stream[bool], channel_name[&str]);
    // cfg.set_tools(&str);
    // cfg.set_system_message(&str);

    //-------
    let engine = Engine::new(&settings)?;
    let conversation = engine.create_conversation_with_config(&cfg)?;
    // Or: create conversation with the default conversation config
    // let conversation = engine.create_conversation();
    //
    // Or with a custom session config
    // let session = engine.create_session(Some(&session_config))?;

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
        let mut printed_thinking = false;
        let result = conversation.send_message_stream(&message_json, |chunk| {
            if let Some(raw) = chunk.text() {
                let thought = extract_thinking(&raw);
                if !thought.is_empty() {
                    print!("{thought}");
                    io::stdout().flush().ok();
                    printed_anything = true;
                    printed_thinking = true;
                } else {
                    if printed_thinking {
                        println!("\n---\n");
                        printed_thinking = false;
                    }
                    let text = extract_text(&raw);
                    if !text.is_empty() {
                        print!("{text}");
                        io::stdout().flush().ok();
                        printed_anything = true;
                    }
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

pub fn extract_thinking(message_json: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(message_json) else {
        return String::new();
    };
    let Some(content) = value.get("channels").and_then(|c| c.as_object()) else {
        return String::new();
    };
    content
        .get("thought")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .into()
}

# litertlm-rs

Safe, idiomatic Rust bindings to Google's [LiteRT LM](https://github.com/google-ai-edge/LiteRT-LM)
C API, built on top of the raw [`litertlm-sys`](https://crates.io/crates/litertlm-sys)
FFI crate.

Covers the full C API (`conversation.h` + `engine.h`) — engine/session/
conversation lifecycle, sampler and decoding-control configs, streaming,
tokenization, and benchmarking — with the core flow:

```text
EngineSettings -> Engine -> Conversation -> send_message[_stream]
```

## Quick example

```rust,no_run
use litertlm_rs::{extract_text, Engine, EngineSettings};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = EngineSettings::new("/path/to/model.litertlm", "cpu", None, None)?;
    let engine = Engine::new(&settings)?;
    let conversation = engine.create_conversation()?;

    let response = conversation.send_message(r#"{"role":"user","content":"Hello!"}"#)?;
    println!("{}", extract_text(&response));
    Ok(())
}
```

More examples (including streaming and interactive chat) are in
[`examples/`](examples).

## Setup

This crate links against Google's official LiteRT-LM C API prebuilt
library. `litertlm-sys` will download the right one for your target
automatically, and this crate's `build.rs` copies it next to your compiled
binary and wires up runtime linking (rpath on Linux/macOS, DLL-next-to-exe
on Windows) — no manual `LD_LIBRARY_PATH` needed.

`bindgen` (a build-time dependency of `litertlm-sys`) needs `libclang`
installed on your machine:

```bash
# Debian/Ubuntu
sudo apt install libclang-dev clang
# Fedora
sudo dnf install clang-devel
# macOS
brew install llvm
```

`litertlm-sys` will download the needed C API prebuilt library
automatically. If you'd rather supply your own downloaded copy instead,
Download the official C API prebuilt package for your platform from
[LiteRT-LM's GitHub releases](https://github.com/google-ai-edge/LiteRT-LM/releases)
(v0.16.0 or later — look for the C API prebuilt asset, not the source
tarball)
Extract it into `your/project/root/prebuilt/`
Add these lines to your project's `.cargo/config.toml` so `litertlm-sys`
knows where to find it:

```toml
[env]
LITERT_LM_LIB_DIR = { value = "prebuilt", relative = true }
```

For the full setup walkthrough — including how to point at a prebuilt
library you downloaded yourself instead of letting it auto-download, and
the details of how runtime linking is wired up — see the
[repository README](https://github.com/kiamazi/LiteRT-LM-rs), which also
has the complete function-by-function API reference.

---

## litertlm-rs Usage Guide

Complete API reference with examples for every struct, enum, and method in the `litertlm-rs` safe Rust wrapper around Google's LiteRT-LM C API.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
litertlm-rs = "0.16"
serde_json = "1"  # for building message JSON
```

Minimal example:

```rust
use litertlm_rs::{Engine, EngineSettings, LogSeverity};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    litertlm_rs::set_min_log_level(LogSeverity::Silent);

    let settings = EngineSettings::new("model.litertlm", "cpu", None, None)?;
    let engine = Engine::new(&settings)?;
    let convo = engine.create_conversation()?;

    let response = convo.send_message(
        r#"{"role":"user","content":"What is 2+2?"}"#
    )?;
    println!("{}", litertlm_rs::extract_text(&response));
    Ok(())
}
```

## Core Types

### LogSeverity

Controls C++ library logging verbosity (must be called before Engine::new):

```rust
use litertlm_rs::{set_min_log_level, LogSeverity};

set_min_log_level(LogSeverity::Silent);    // No output at all
set_min_log_level(LogSeverity::Error);     // Only errors
set_min_log_level(LogSeverity::Warning);   // Errors + warnings
set_min_log_level(LogSeverity::Info);      // Errors + warnings + info
set_min_log_level(LogSeverity::Debug);     // All except verbose
set_min_log_level(LogSeverity::Verbose);   // Everything
```

### Error

All API methods return `Result<T, Error>`:

```rust
use litertlm_rs::{Error, Engine, EngineSettings};

let settings = EngineSettings::new("model.litertlm", "cpu", None, None)?;
match Engine::new(&settings) {
    Ok(engine) => println!("Loaded successfully"),
    Err(Error::NulArgument(name)) => eprintln!("NUL byte in argument: {}", name),
    Err(Error::CreateFailed(what)) => eprintln!("C API returned NULL: {}", what),
    Err(Error::CallFailed(what)) => eprintln!("C API returned error: {}", what),
    Err(Error::Stream(msg)) => eprintln!("Stream error: {}", msg),
    Err(e) => eprintln!("Unexpected error: {}", e),
}
```

### extract_text (helper function)

Extracts plain text from a message JSON object's `content` blocks:

```rust
let response = convo.send_message(r#"{"role":"user","content":"Hi"}"#)?;
// response may be: {"role":"assistant","content":[{"type":"text","text":"Hello!"}]}
let text = litertlm_rs::extract_text(&response);
// text == "Hello!"
```

### SamplerType

Selects token sampling strategy:

```rust
use litertlm_rs::{SamplerType, SamplerParams};

let greedy = SamplerParams::new(SamplerType::Greedy)?; // argmax, deterministic
let top_k = SamplerParams::new(SamplerType::TopK)?;    // sample from top k
let top_p = SamplerParams::new(SamplerType::TopP)?;    // nucleus sampling
```

### ConstraintType

Output constraint type for constrained decoding:

```rust
use litertlm_rs::ConstraintType;

let none = ConstraintType::None;
let regex = ConstraintType::Regex;
let json_schema = ConstraintType::JsonSchema;
```

### ConstraintProviderType

Selects constraint enforcement backend:

```rust
use litertlm_rs::ConstraintProviderType;

let provider = ConstraintProviderType::LlGuidance;
```

### InputDataType

Type of input data for Session API:

```rust
use litertlm_rs::{InputData, InputDataType};

let text = InputData::new(InputDataType::Text, b"Hello, model!")?;
let image = InputData::new(InputDataType::Image, &image_bytes)?;
let image_end = InputData::new(InputDataType::ImageEnd, &[])?;
let audio = InputData::new(InputDataType::Audio, &audio_bytes)?;
let audio_end = InputData::new(InputDataType::AudioEnd, &[])?;
```

### ActivationDataType

Precision for internal activations:

```rust
use litertlm_rs::{ActivationDataType, EngineSettings};

let mut settings = EngineSettings::new("model.litertlm", "cpu", None, None)?;
settings.set_activation_data_type(ActivationDataType::Float32); // Default
settings.set_activation_data_type(ActivationDataType::Float16); // Less memory
settings.set_activation_data_type(ActivationDataType::Int16);    // Quantized
settings.set_activation_data_type(ActivationDataType::Int8);     // Most quantized
```

### TokenUnionType

Type of a token union (BOS/EOS tokens):

```rust
use litertlm_rs::TokenUnionType;

match token_union.union_type() {
    TokenUnionType::String => {
        println!("Token as string: {}", token_union.as_string().unwrap());
    }
    TokenUnionType::Ids => {
        println!("Token as IDs: {:?}", token_union.as_ids());
    }
}
```

---

## EngineSettings

Builder for engine configuration. Wraps `LiteRtLmEngineSettings`.

```rust
use litertlm_rs::{EngineSettings, ActivationDataType};

let mut settings = EngineSettings::new("model.litertlm", "cpu", None, None)?;
settings
    .set_num_threads(4)
    .set_max_num_tokens(4096)
    .set_activation_data_type(ActivationDataType::Float16)
    .set_cache_dir("/tmp/litertlm_cache")
    .set_parallel_file_section_loading(true);

let engine = Engine::new(&settings)?;
```

| Method                                | Signature                                                                     | Description                                                             |
| ------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| `new`                                 | `(model_path, backend, vision_backend, audio_backend) -> Result<Self, Error>` | Creates engine settings. Backends: `"cpu"`, `"gpu"`, or `None`.         |
| `from_raw_file_descriptor`            | `(fd, backend, vision, audio) -> Result<Self, Error>`                         | Creates settings from an open file descriptor (engine takes ownership). |
| `set_max_num_tokens`                  | `(&mut self, max) -> &mut Self`                                               | Maximum context length in tokens.                                       |
| `set_num_threads`                     | `(&mut self, n) -> &mut Self`                                                 | CPU backend thread count.                                               |
| `set_audio_num_threads`               | `(&mut self, n) -> &mut Self`                                                 | Audio CPU backend thread count.                                         |
| `set_parallel_file_section_loading`   | `(&mut self, parallel) -> &mut Self`                                          | Load file sections in parallel (default: true).                         |
| `set_max_num_images`                  | `(&mut self, max) -> &mut Self`                                               | Max images for legacy engine.                                           |
| `set_cache_dir`                       | `(&mut self, dir) -> Result<&mut Self, Error>`                                | On-disk cache directory.                                                |
| `set_litert_dispatch_lib_dir`         | `(&mut self, dir) -> Result<&mut Self, Error>`                                | NPU dispatch library directory.                                         |
| `set_activation_data_type`            | `(&mut self, dt) -> &mut Self`                                                | Activation dtype (fp32/fp16/int16/int8).                                |
| `set_prefill_chunk_size`              | `(&mut self, size) -> &mut Self`                                              | CPU dynamic model prefill chunk size.                                   |
| `set_enable_ynnpack`                  | `(&mut self, enable) -> &mut Self`                                            | Allow YNNPACK delegation.                                               |
| `enable_benchmark`                    | `(&mut self) -> &mut Self`                                                    | Enable benchmark data collection.                                       |
| `set_num_prefill_tokens`              | `(&mut self, n) -> &mut Self`                                                 | Prefill tokens for benchmarking.                                        |
| `set_num_decode_tokens`               | `(&mut self, n) -> &mut Self`                                                 | Decode tokens for benchmarking.                                         |
| `set_enable_speculative_decoding`     | `(&mut self, enable) -> &mut Self`                                            | Enable speculative decoding.                                            |
| `set_gpu_decode_steps_per_sync`       | `(&mut self, steps) -> &mut Self`                                             | GPU decode sync frequency.                                              |
| `set_gpu_wait_for_weight_uploads`     | `(&mut self, wait) -> &mut Self`                                              | Wait for GPU weight uploads.                                            |
| `set_use_ringbuffers_local_attention` | `(&mut self, use_ring) -> &mut Self`                                          | Ringbuffer KV cache (GPU Artisan only).                                 |
| `set_lora_rank`                       | `(&mut self, rank) -> &mut Self`                                              | Text LoRA rank.                                                         |
| `set_supported_lora_ranks`            | `(&mut self, ranks) -> Result<&mut Self, Error>`                              | Supported text LoRA ranks.                                              |
| `set_audio_lora_rank`                 | `(&mut self, rank) -> &mut Self`                                              | Audio LoRA rank.                                                        |
| `set_supported_audio_lora_ranks`      | `(&mut self, ranks) -> Result<&mut Self, Error>`                              | Supported audio LoRA ranks.                                             |

---

## Engine

Main entry point — loads a model and creates sessions/conversations.

```rust
use litertlm_rs::{Engine, EngineSettings};

let settings = EngineSettings::new("model.litertlm", "cpu", None, None)?;
let engine = Engine::new(&settings)?;

// Create a high-level conversation (manages KV cache)
let convo = engine.create_conversation()?;

// Create a low-level session (manual prefill/decode control)
let session = engine.create_session(None)?;

// Or with a custom session config
let session = engine.create_session(Some(&session_config))?;
```

| Method                            | Description                                                                                                           |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `new`                             | `(settings: &EngineSettings) -> Result<Self, Error>` — Creates the engine (loads the model).                          |
| `create_session`                  | `(config: Option<&SessionConfig>) -> Result<Session, Error>` — Creates a low-level session. Pass `None` for defaults. |
| `create_conversation`             | `() -> Result<Conversation, Error>` — Creates a conversation with default config.                                     |
| `create_conversation_with_config` | `(config: &ConversationConfig) -> Result<Conversation, Error>` — Creates a conversation with custom config.           |
| `tokenize`                        | `(text: &str) -> Result<TokenizeResult, Error>` — Tokenizes text using the model's tokenizer.                         |
| `detokenize`                      | `(tokens: &[i32]) -> Result<DetokenizeResult, Error>` — Converts token IDs back to text.                              |
| `get_start_token`                 | `() -> Option<TokenUnion>` — Returns the BOS (start) token, if configured.                                            |
| `get_stop_tokens`                 | `() -> Option<TokenUnions>` — Returns the EOS (stop) tokens, if configured.                                           |

---

## Session (low-level prefill/decode API)

```rust
use litertlm_rs::{SessionConfig, InputData, InputDataType};

let session = engine.create_session(Some(&session_config))?;

// 1. Prefill (blocking — feeds input into the model)
let inputs = vec![InputData::text("Explain this:")?, /* image/audio data */];
session.run_prefill(&inputs)?;

// 2a. Blocking decode
let responses = session.run_decode()?;
println!("{}", responses.response_text_at(0).unwrap_or_default());

// One-shot: prefill + decode in one call
let responses = session.generate_content(&[InputData::text("What is 2+2?")?])?;

// Streaming decode
session.run_decode_async(|chunk| {
    print!("{}", litertlm_rs::extract_text(chunk.text().unwrap_or_default()));
})?;

// Streaming generate content
session.generate_content_stream(
    &[InputData::text("Write a poem:")?],
    |chunk| { /* handle chunk */ }
)?;
```

| Method                    | Description                                                                                        |
| ------------------------- | -------------------------------------------------------------------------------------------------- |
| `cancel_process`          | `()` — Cancels in-flight processing.                                                               |
| `save_checkpoint`         | `(label: &str) -> Result<(), Error>` — Saves current session state. v0.16.0+ required.             |
| `rewind_to_checkpoint`    | `(label: &str) -> Result<(), Error>` — Rewinds to a saved checkpoint. v0.16.0+ required.           |
| `rewind_to_step`          | `(step: i32) -> Result<(), Error>` — Rewinds to a specific decode step. v0.16.0+ required.         |
| `run_prefill`             | `(inputs: &[InputData]) -> Result<(), Error>` — Blocking: feeds input for prefill.                 |
| `run_decode`              | `() -> Result<Responses, Error>` — Blocking: decodes response after prefill.                       |
| `run_text_scoring`        | `(targets: &[&str], store_lengths: bool) -> Result<Responses, Error>` — Scores target texts.       |
| `generate_content`        | `(inputs: &[InputData]) -> Result<Responses, Error>` — Blocking: prefill + decode in one call.     |
| `run_decode_async`        | `(on_chunk: impl FnMut(&StreamChunk)) -> Result<(), Error>` — Streaming decode.                    |
| `generate_content_stream` | `(inputs: &[InputData], on_chunk) -> Result<(), Error>` — Streaming: prefill + decode in one call. |
| `get_benchmark_info`      | `() -> Result<BenchmarkInfo, Error>` — Retrieves benchmark data.                                   |

---

## Conversation (high-level chat API)

```rust
use litertlm_rs::{ConversationConfig, Engine, EngineSettings};

let engine = Engine::new(&settings)?;

// Simple conversation with defaults
let convo = engine.create_conversation()?;

// Customized conversation
let mut cfg = ConversationConfig::new()?;
cfg.set_max_output_tokens(2048);
let convo = engine.create_conversation_with_config(&cfg)?;

// Send a message (blocking)
let response = convo.send_message(
    r#"{"role":"user","content":"Hello!"}"#
)?;
println!("{}", litertlm_rs::extract_text(&response));

// Stream a response
convo.send_message_stream(
    r#"{"role":"user","content":"Tell me a story"}"#,
    |chunk| {
        if let Some(text) = chunk.text() {
            print!("{}", litertlm_rs::extract_text(&text));
        }
    },
)?;

// Clone conversation (including KV cache state)
let clone = convo.clone_conversation()?;

// Get token count
let tokens = convo.token_count()?;

// Cancel in-flight processing
convo.cancel_process();
```

| Method                          | Description                                                                                                   |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| `clone_conversation`            | `() -> Result<Conversation, Error>` — Duplicates the conversation including KV cache.                         |
| `send_message`                  | `(message_json: &str) -> Result<String, Error>` — Blocking: sends a message, returns full JSON response.      |
| `send_message_with_args`        | `(message, extra_context, optional_args) -> Result<String, Error>` — With optional overrides.                 |
| `send_message_stream`           | `(message_json, on_chunk) -> Result<(), Error>` — Streaming: sends a message, calls callback per chunk.       |
| `send_message_stream_with_args` | `(message, extra_context, optional_args, on_chunk) -> Result<(), Error>` — Streaming with optional overrides. |
| `render_message_to_string`      | `(message_json: &str) -> Result<String, Error>` — Renders a message through the prompt template.              |
| `render_preface_to_string`      | `() -> Result<String, Error>` — Renders the conversation preface (system message, tools, etc.).               |
| `cancel_process`                | `()` — Cancels in-flight processing.                                                                          |
| `get_benchmark_info`            | `() -> Result<BenchmarkInfo, Error>` — Retrieves benchmark data.                                              |
| `token_count`                   | `() -> Result<i32, Error>` — Returns token count in the KV cache.                                             |

---

## ConversationConfig

Builder for conversation-level configuration:

```rust
use litertlm_rs::{ConversationConfig, SessionConfig, ThinkingConfig};

let mut session = SessionConfig::new()?;
let mut thinking = ThinkingConfig::new()?;

let mut cfg = ConversationConfig::new()?;
cfg
    .set_session_config(&session)
    .set_system_message(r#"{"role":"system","content":"You are a helpful assistant."}"#)?
    .set_thinking_config(&thinking);
```

| Method                                     | Signature                                                              | Description                                           |
| ------------------------------------------ | ---------------------------------------------------------------------- | ----------------------------------------------------- |
| `new`                                      | `() -> Result<Self, Error>`                                            | Creates an empty conversation config.                 |
| `set_session_config`                       | `(&mut self, session: &SessionConfig) -> &mut Self`                    | Attaches a SessionConfig (sampler, max tokens, etc.). |
| `set_system_message`                       | `(&mut self, json: &str) -> Result<&mut Self, Error>`                  | Sets the system message JSON.                         |
| `set_tools`                                | `(&mut self, json: &str) -> Result<&mut Self, Error>`                  | Sets tools JSON array for function calling.           |
| `set_messages`                             | `(&mut self, json: &str) -> Result<&mut Self, Error>`                  | Seeds conversation with initial messages.             |
| `set_extra_context`                        | `(&mut self, json: &str) -> Result<&mut Self, Error>`                  | Sets extra context injected into the preface.         |
| `set_prompt_template`                      | `(&mut self, template: &str) -> Result<&mut Self, Error>`              | Overrides the default prompt template.                |
| `set_enable_constrained_decoding`          | `(&mut self, enable: bool) -> &mut Self`                               | Enables constrained decoding.                         |
| `set_constraint_provider`                  | `(&mut self, provider: Option<ConstraintProviderType>) -> &mut Self`   | Sets constraint provider backend.                     |
| `set_filter_channel_content_from_kv_cache` | `(&mut self, filter: bool) -> &mut Self`                               | Filter thinking channel content from KV cache.        |
| `set_stream_tool_calls`                    | `(&mut self, stream: bool, channel: &str) -> Result<&mut Self, Error>` | Toggle streaming tool calls on a named channel.       |
| `set_thinking_config`                      | `(&mut self, thinking: &ThinkingConfig) -> &mut Self`                  | Attaches a ThinkingConfig for reasoning models.       |

---

## ConversationOptionalArgs (per-turn overrides)

```rust
use litertlm_rs::{ConversationOptionalArgs, RepetitionPenaltyConfig};

let mut args = ConversationOptionalArgs::new()?;

let mut penalty = RepetitionPenaltyConfig::new()?;
penalty.set_repetition_penalty(1.3);
args.set_repetition_penalty_config(&penalty);

args.set_max_output_tokens(128);

convo.send_message_with_args(message_json, None, Some(&args))?;
```

| Method                          | Signature                                                                                           | Description                                        |
| ------------------------------- | --------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| `new`                           | `() -> Result<Self, Error>`                                                                         | Creates an empty optional args object.             |
| `set_repetition_penalty_config` | `(&mut self, config: &RepetitionPenaltyConfig) -> &mut Self`                                        | Sets repetition penalty for this turn.             |
| `set_no_repeat_ngram_config`    | `(&mut self, config: &NoRepeatNgramConfig) -> &mut Self`                                            | Sets no-repeat n-gram config for this turn.        |
| `set_suppress_tokens_config`    | `(&mut self, config: &SuppressTokensConfig) -> &mut Self`                                           | Suppresses specific tokens for this turn.          |
| `set_visual_token_budget`       | `(&mut self, budget: i32) -> &mut Self`                                                             | Caps vision tokens for this turn.                  |
| `set_max_output_tokens`         | `(&mut self, max: i32) -> &mut Self`                                                                | Caps output tokens for this turn.                  |
| `set_thinking_config`           | `(&mut self, thinking: &ThinkingConfig) -> &mut Self`                                               | Overrides thinking config for this turn.           |
| `set_constraint`                | `(&mut self, constraint_type: ConstraintType, constraint_string: &str) -> Result<&mut Self, Error>` | Sets a regex/JSON-schema constraint for this turn. |

---

## SessionConfig

Builder for session-level configuration (sampling, output limits, LoRAs):

```rust
use litertlm_rs::{SessionConfig, SamplerParams, SamplerType};

let mut sampler = SamplerParams::new(SamplerType::TopP)?;
sampler.set_top_p(0.95).set_temperature(0.7);

let mut cfg = SessionConfig::new()?;
cfg
    .set_sampler_params(&sampler)
    .set_max_output_tokens(2048)
    .set_apply_prompt_template(true);
```

| Method                      | Signature                                             | Description                                 |
| --------------------------- | ----------------------------------------------------- | ------------------------------------------- |
| `new`                       | `() -> Result<Self, Error>`                           | Creates default session config.             |
| `set_max_output_tokens`     | `(&mut self, max: i32) -> &mut Self`                  | Caps output tokens per decode step.         |
| `set_apply_prompt_template` | `(&mut self, apply: bool) -> &mut Self`               | Enable/disable automatic prompt templating. |
| `set_sampler_params`        | `(&mut self, params: &SamplerParams) -> &mut Self`    | Sets the sampler parameters.                |
| `set_lora_path`             | `(&mut self, path: &str) -> Result<&mut Self, Error>` | Sets the text LoRA weights path.            |
| `set_audio_lora_path`       | `(&mut self, path: &str) -> Result<&mut Self, Error>` | Sets the audio LoRA weights path.           |

---

## SamplerParams

Controls token sampling behavior (top-k, top-p, temperature, seed):

```rust
use litertlm_rs::{SamplerParams, SamplerType};

let mut sampler = SamplerParams::new(SamplerType::TopP)?;
sampler
    .set_top_k(100)        // Consider top 100 tokens
    .set_top_p(0.9)        // Nucleus sampling at 90%
    .set_temperature(0.7)  // Some randomness
    .set_seed(42);         // Deterministic output
```

| Method            | Signature                                            | Description                                          |
| ----------------- | ---------------------------------------------------- | ---------------------------------------------------- |
| `new`             | `(sampler_type: SamplerType) -> Result<Self, Error>` | Creates sampler params with the given sampler type.  |
| `set_top_k`       | `(&mut self, k: i32) -> &mut Self`                   | Sets the top-k value.                                |
| `set_top_p`       | `(&mut self, p: f32) -> &mut Self`                   | Sets the top-p (nucleus) value.                      |
| `set_temperature` | `(&mut self, temp: f32) -> &mut Self`                | Sets the sampling temperature (0.0 = deterministic). |
| `set_seed`        | `(&mut self, seed: i32) -> &mut Self`                | Sets the RNG seed for deterministic output.          |

---

## RepetitionPenaltyConfig

Controls repetition suppression (multiplicative + subtractive penalties):

```rust
use litertlm_rs::RepetitionPenaltyConfig;

let mut penalty = RepetitionPenaltyConfig::new()?;
penalty
    .set_repetition_penalty(1.2)  // Multiply logits by 1/1.2 for seen tokens
    .set_presence_penalty(0.5)    // Subtract 0.5 for each distinct token seen
    .set_frequency_penalty(0.3)   // Subtract 0.3 × count for each occurrence
    .set_window_size(64);         // Only look at last 64 tokens
```

| Method                   | Signature                               | Description                                           |
| ------------------------ | --------------------------------------- | ----------------------------------------------------- |
| `new`                    | `() -> Result<Self, Error>`             | Creates with defaults (all penalties disabled).       |
| `set_repetition_penalty` | `(&mut self, value: f32) -> &mut Self`  | Multiplicative penalty (>1.0 reduces repetition).     |
| `set_presence_penalty`   | `(&mut self, value: f32) -> &mut Self`  | Flat subtractive penalty per distinct token.          |
| `set_frequency_penalty`  | `(&mut self, value: f32) -> &mut Self`  | Frequency-scaled subtractive penalty.                 |
| `set_window_size`        | `(&mut self, window: i32) -> &mut Self` | Token window for penalty calculation (0 = unlimited). |

---

## NoRepeatNgramConfig

Ban repeating n-grams during generation:

```rust
use litertlm_rs::NoRepeatNgramConfig;

let mut ngram = NoRepeatNgramConfig::new()?;
ngram
    .set_no_repeat_ngram_size(3)  // Ban repeating 3-grams
    .set_window_size(64);         // Look at last 64 tokens
```

| Method                     | Signature                               | Description                                         |
| -------------------------- | --------------------------------------- | --------------------------------------------------- |
| `new`                      | `() -> Result<Self, Error>`             | Creates with defaults (size=0, window=0, disabled). |
| `set_no_repeat_ngram_size` | `(&mut self, size: i32) -> &mut Self`   | N-gram length to ban from repeating.                |
| `set_window_size`          | `(&mut self, window: i32) -> &mut Self` | Token window for n-gram checking.                   |

---

## SuppressTokensConfig

Suppress specific token IDs from being generated:

```rust
use litertlm_rs::SuppressTokensConfig;

let mut suppress = SuppressTokensConfig::new()?;
suppress.set_suppress_tokens(&[1, 2, 3]);  // Suppress tokens 1, 2, 3
```

| Method                | Signature                                  | Description                                              |
| --------------------- | ------------------------------------------ | -------------------------------------------------------- |
| `new`                 | `() -> Result<Self, Error>`                | Creates with defaults (empty set, suppression disabled). |
| `set_suppress_tokens` | `(&mut self, tokens: &[i32]) -> &mut Self` | Set token IDs to suppress (empty slice clears).          |

---

## ThinkingConfig

Configure reasoning/thinking behavior for thinking models:

```rust
use litertlm_rs::ThinkingConfig;

let mut thinking = ThinkingConfig::new()?;
thinking
    .set_enable_thinking(true)
    .set_thinking_token_budget(-1);  // Unlimited thinking budget
```

| Method                      | Signature                                | Description                                                |
| --------------------------- | ---------------------------------------- | ---------------------------------------------------------- |
| `new`                       | `() -> Result<Self, Error>`              | Creates with defaults (thinking enabled, infinite budget). |
| `set_enable_thinking`       | `(&mut self, enable: bool) -> &mut Self` | Enable/disable thinking/reasoning.                         |
| `set_thinking_token_budget` | `(&mut self, budget: i32) -> &mut Self`  | Max thinking tokens (-1 = infinite).                       |

---

## InputData

Multimodal input chunk (text, image, or audio):

```rust
use litertlm_rs::{InputData, InputDataType};

// Text
let text = InputData::text("Describe this image:")?;

// Image (raw bytes)
let image = InputData::new(InputDataType::Image, &image_bytes)?;
let image_end = InputData::new(InputDataType::ImageEnd, &[])?;

// Audio (raw bytes)
let audio = InputData::new(InputDataType::Audio, &audio_bytes)?;
let audio_end = InputData::new(InputDataType::AudioEnd, &[])?;
```

| Method | Signature                                                        | Description                             |
| ------ | ---------------------------------------------------------------- | --------------------------------------- |
| `new`  | `(data_type: InputDataType, data: &[u8]) -> Result<Self, Error>` | Creates input data from raw bytes.      |
| `text` | `(text: &str) -> Result<Self, Error>`                            | Convenience constructor for text input. |

---

## Responses

Result of `Session::run_decode`, `run_text_scoring`, or `generate_content`:

```rust
let responses = session.run_decode()?;

// Number of candidate responses
println!("Candidates: {}", responses.num_candidates());

// Text of candidate 0
println!("Text: {}", responses.response_text_at(0).unwrap_or_default());

// Score (if available — from text scoring)
if let Some(score) = responses.score_at(0) {
    println!("Score: {}", score);
}

// Token count (if available)
if let Some(len) = responses.token_length_at(0) {
    println!("Tokens: {}", len);
}

// Per-token scores (if available)
if let Some(scores) = responses.token_scores_at(0) {
    println!("Per-token scores: {:?}", scores);
}
```

| Method             | Signature                                 | Description                                      |
| ------------------ | ----------------------------------------- | ------------------------------------------------ |
| `num_candidates`   | `(&self) -> i32`                          | Returns the number of response candidates.       |
| `response_text_at` | `(&self, index: i32) -> Option<String>`   | Returns the text of candidate at index.          |
| `score_at`         | `(&self, index: i32) -> Option<f32>`      | Returns the score for candidate at index.        |
| `token_length_at`  | `(&self, index: i32) -> Option<i32>`      | Returns the token length for candidate at index. |
| `token_scores_at`  | `(&self, index: i32) -> Option<Vec<f32>>` | Returns per-token scores for candidate at index. |

---

## BenchmarkInfo

Performance metrics (requires `EngineSettings::enable_benchmark()`):

```rust
let mut settings = EngineSettings::new("model.litertlm", "cpu", None, None)?;
settings.enable_benchmark();
settings.set_num_prefill_tokens(64);
settings.set_num_decode_tokens(32);

let engine = Engine::new(&settings)?;
let convo = engine.create_conversation()?;

convo.send_message(r#"{"role":"user","content":"Hello"}"#)?;

let bench = convo.get_benchmark_info()?;
println!("TTFT: {}s", bench.time_to_first_token());
println!("Init time: {}s", bench.total_init_time_in_second());
println!("Prefill turns: {}", bench.num_prefill_turns());
println!("Decode turns: {}", bench.num_decode_turns());

for i in 0..bench.num_prefill_turns() {
    println!(
        "  Turn {}: {} prefill tokens ({} tok/s), {} decode tokens ({} tok/s)",
        i,
        bench.prefill_token_count_at(i),
        bench.prefill_tokens_per_sec_at(i),
        bench.decode_token_count_at(i),
        bench.decode_tokens_per_sec_at(i),
    );
}
```

| Method                      | Signature                    | Description                                        |
| --------------------------- | ---------------------------- | -------------------------------------------------- |
| `time_to_first_token`       | `(&self) -> f64`             | Seconds from prefill start to first decoded token. |
| `total_init_time_in_second` | `(&self) -> f64`             | Total engine initialization time.                  |
| `num_prefill_turns`         | `(&self) -> i32`             | Number of prefill turns recorded.                  |
| `num_decode_turns`          | `(&self) -> i32`             | Number of decode turns recorded.                   |
| `prefill_token_count_at`    | `(&self, index: i32) -> i32` | Prefill token count for turn at index.             |
| `decode_token_count_at`     | `(&self, index: i32) -> i32` | Decode token count for turn at index.              |
| `prefill_tokens_per_sec_at` | `(&self, index: i32) -> f64` | Prefill throughput for turn at index.              |
| `decode_tokens_per_sec_at`  | `(&self, index: i32) -> f64` | Decode throughput for turn at index.               |

---

## StreamChunk

Borrowed view of a single streaming chunk during streaming operations:

```rust
convo.send_message_stream(
    r#"{"role":"user","content":"Tell me a joke"}"#,
    |chunk| {
        // Text content of this chunk
        if let Some(text) = chunk.text() {
            print!("{}", litertlm_rs::extract_text(&text));
        }

        // Is this the final chunk?
        if chunk.is_final() {
            println!(); // response complete
        }

        // Error (if any)
        if let Some(err) = chunk.error() {
            eprintln!("Stream error: {}", err);
        }
    },
)?;
```

| Method     | Signature                   | Description                                     |
| ---------- | --------------------------- | ----------------------------------------------- |
| `text`     | `(&self) -> Option<String>` | Returns the JSON text content of the chunk.     |
| `is_final` | `(&self) -> bool`           | Whether this is the last chunk.                 |
| `error`    | `(&self) -> Option<String>` | Returns the error message if the stream failed. |

---

## TokenizeResult

Result of `Engine::tokenize`:

```rust
let result = engine.tokenize("Hello, world!")?;
let tokens: Vec<i32> = result.tokens();
println!("Tokens: {:?}", tokens);
```

| Method   | Signature             | Description                           |
| -------- | --------------------- | ------------------------------------- |
| `tokens` | `(&self) -> Vec<i32>` | Returns token IDs as an owned vector. |

---

## DetokenizeResult

Result of `Engine::detokenize`:

```rust
let result = engine.detokenize(&[1, 1234, 5678, 2])?;
println!("Text: {}", result.text().unwrap_or_default());
```

| Method | Signature                   | Description                   |
| ------ | --------------------------- | ----------------------------- |
| `text` | `(&self) -> Option<String>` | Returns the detokenized text. |

---

## TokenUnion

Represents a single BOS/EOS token (string or ID sequence):

```rust
let engine = Engine::new(&settings)?;

if let Some(bos) = engine.get_start_token() {
    println!("BOS type: {:?}", bos.union_type());
    if let Some(s) = bos.as_string() {
        println!("BOS string: {}", s);
    }
    if let Some(ids) = bos.as_ids() {
        println!("BOS ids: {:?}", ids);
    }
}
```

| Method       | Signature                     | Description                              |
| ------------ | ----------------------------- | ---------------------------------------- |
| `union_type` | `(&self) -> TokenUnionType`   | Whether this holds a string or IDs.      |
| `as_string`  | `(&self) -> Option<String>`   | Returns the string value, if applicable. |
| `as_ids`     | `(&self) -> Option<Vec<i32>>` | Returns token IDs, if applicable.        |

---

## TokenUnions

Collection of token unions (typically stop/EOS tokens):

```rust
if let Some(stops) = engine.get_stop_tokens() {
    println!("Stop tokens: {}", stops.len());
    for i in 0..stops.len() {
        if let Some(token) = stops.get(i) {
            if let Some(s) = token.as_string() {
                println!("  Stop string: {}", s);
            }
        }
    }
}
```

| Method     | Signature                                     | Description                       |
| ---------- | --------------------------------------------- | --------------------------------- |
| `len`      | `(&self) -> usize`                            | Number of token unions.           |
| `is_empty` | `(&self) -> bool`                             | Whether there are no tokens.      |
| `get`      | `(&self, index: usize) -> Option<TokenUnion>` | Returns the token union at index. |

---

## Complete Examples

### Basic chat

```rust
use litertlm_rs::{Engine, EngineSettings, LogSeverity};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    litertlm_rs::set_min_log_level(LogSeverity::Silent);

    let settings = EngineSettings::new("model.litertlm", "cpu", None, None)?;
    let engine = Engine::new(&settings)?;
    let convo = engine.create_conversation()?;

    let response = convo.send_message(
        r#"{"role":"user","content":"What is the capital of France?"}"#
    )?;
    println!("{}", litertlm_rs::extract_text(&response));
    Ok(())
}
```

### Interactive streaming chat with thinking

```rust
use litertlm_rs::*;
use std::io::{self, BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let model_path = &args[1];

    let mut settings = EngineSettings::new(model_path, "cpu", None, None)?;
    settings.set_num_threads(4);

    let engine = Engine::new(&settings)?;

    let mut sampler = SamplerParams::new(SamplerType::TopP)?;
    sampler.set_top_p(0.9).set_temperature(0.7);

    let mut session = SessionConfig::new()?;
    session.set_sampler_params(&sampler).set_max_output_tokens(8192);

    let mut thinking = ThinkingConfig::new()?;
    thinking.set_enable_thinking(true).set_thinking_token_budget(-1);

    let mut cfg = ConversationConfig::new()?;
    cfg.set_session_config(&session);
    cfg.set_thinking_config(&thinking);

    let convo = engine.create_conversation_with_config(&cfg)?;

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 { break; }
        let prompt = line.trim();
        if prompt.is_empty() { break; }

        let message = serde_json::json!({ "role": "user", "content": prompt }).to_string();
        convo.send_message_stream(&message, |chunk| {
            if let Some(raw) = chunk.text() {
                let text = litertlm_rs::extract_text(&raw);
                print!("{}", text);
                io::stdout().flush().ok();
            }
        })?;
        println!();
    }
    Ok(())
}
```

### Constrained output example

```rust
use litertlm_rs::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = EngineSettings::new("model.litertlm", "cpu", None, None)?;
    let engine = Engine::new(&settings)?;

    let mut cfg = ConversationConfig::new()?;
    cfg.set_enable_constrained_decoding(true);
    cfg.set_constraint_provider(ConstraintProviderType::LlGuidance);

    let convo = engine.create_conversation_with_config(&cfg)?;

    let mut args = ConversationOptionalArgs::new()?;
    args.set_constraint(ConstraintType::Regex, r#"(yes|no)"#)?;

    let response = convo.send_message_with_args(
        r#"{"role":"user","content":"Are you an AI?"}"#,
        None,
        Some(&args),
    )?;
    println!("{}", litertlm_rs::extract_text(&response));
    Ok(())
}
```

## See Also

- [`examples/basic.rs`](examples/basic.rs) — Hello world
- [`examples/streaming.rs`](examples/streaming.rs) — Streaming output
- [`examples/interactive.rs`](examples/interactive.rs) — Interactive chat REPL with thinking
- [`examples/sandbox_tools.rs`](examples/sandbox_tools.rs) — Tool calling with sandboxed file access
- [`examples/complex.rs`](examples/more_configs.rs) — configuration with thinking, sampler, and constraints

---

## Notes

- **Thread safety:** the C API's thread-safety story isn't documented
  upstream, so types here are `Send` but not `Sync` — don't share an
  `Engine`/`Conversation` across threads concurrently without your own
  synchronization.
- **Message JSON schema:** `send_message` / `send_message_stream` take a
  JSON string, e.g. `{"role": "user", "content": "..."}`; `content` can
  also be an array of multimodal parts (text/image/audio). See
  [LiteRT-LM's docs](https://github.com/google-ai-edge/LiteRT-LM/blob/main/docs/api/cpp/conversation.md)
  for the full schema.

## API coverage

`litertlm-rs`'s safe API wraps every exported `litert_lm_*` function across
`conversation.h` and `engine.h` — the full function-by-function list is at
the bottom of this README. Streaming (`send_message_stream`,
`Session::run_decode_async`, `Session::generate_content_stream`) is
callback-based but converted into a blocking call internally so your
callback closure's lifetime is sound — see the `run_stream` comment in
`litertlm-rs/src/lib.rs` for why.

New in the v0.16.0 C API, also wrapped here: `Session::save_checkpoint` /
`rewind_to_checkpoint` / `rewind_to_step` (save and rewind session state to
a labeled point or a specific step), and
`EngineSettings::set_enable_ynnpack`.

## Notes / gotchas

- **Thread safety:** the C API's thread-safety story isn't documented in
  the header, so this wrapper marks its types `Send` but _not_ `Sync` —
  don't share an `Engine`/`Conversation` across threads concurrently
  without your own synchronization.
- **Message JSON schema:** `send_message` / `send_message_stream` take a
  JSON string (`message_json`). For plain text: `{"role": "user",
"content": "..."}`. For multimodal input, `content` can be an array of
  parts instead, e.g. `[{"type": "text", "text": "..."}, {"type": "image",
"path": "/abs/path.jpg"}]` — see
  [LiteRT-LM's own docs](https://github.com/google-ai-edge/LiteRT-LM/blob/main/docs/api/cpp/conversation.md)
  for the full schema (also supports `"blob"` base64 data instead of
  `"path"`, and `"audio"` parts the same way as `"image"`).
- **Streaming callback safety:** the raw C function
  (`litert_lm_conversation_send_message_stream`) is asynchronous and calls
  back from a background thread. This wrapper blocks until the final chunk
  arrives so your Rust closure doesn't need `'static` — if you want true
  fire-and-forget streaming instead, you'll need to `Box` your callback
  state and manage its lifetime yourself (leak it, or use an `Arc` +
  atomic "done" flag you poll).

## License

Apache-2.0.

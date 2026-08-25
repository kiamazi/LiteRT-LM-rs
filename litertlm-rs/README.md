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

For the full setup walkthrough — including how to point at a prebuilt
library you downloaded yourself instead of letting it auto-download, and
the details of how runtime linking is wired up — see the
[repository README](https://github.com/kiamazi/LiteRT-LM-rs), which also
has the complete function-by-function API reference.

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

## Full API reference

Every `litert_lm_*` function from `engine.h`, grouped by the Rust type that
wraps it, with a one-line summary of what it does.

### Logging / helpers

| Function                      | What it does                                                                          |
| ----------------------------- | ------------------------------------------------------------------------------------- |
| `litert_lm_set_min_log_level` | Sets the underlying C++ library's minimum stderr log severity (`set_min_log_level`).  |
| — (no C function; pure Rust)  | `extract_text` — pulls just the text out of a message JSON object's `content` blocks. |

### `SamplerParams`

| Function                                   | What it does                                                                   |
| ------------------------------------------ | ------------------------------------------------------------------------------ |
| `litert_lm_sampler_params_create`          | Creates sampler parameters for a given sampler type (top-k, top-p, or greedy). |
| `litert_lm_sampler_params_delete`          | Frees a sampler params object.                                                 |
| `litert_lm_sampler_params_set_top_k`       | Sets the top-k value.                                                          |
| `litert_lm_sampler_params_set_top_p`       | Sets the top-p (nucleus) value.                                                |
| `litert_lm_sampler_params_set_temperature` | Sets the sampling temperature.                                                 |
| `litert_lm_sampler_params_set_seed`        | Sets the RNG seed.                                                             |

### `RepetitionPenaltyConfig`

| Function                                                     | What it does                                                                      |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| `litert_lm_repetition_penalty_config_create`                 | Creates a repetition-penalty config (multiplicative + subtractive penalties).     |
| `litert_lm_repetition_penalty_config_delete`                 | Frees it.                                                                         |
| `litert_lm_repetition_penalty_config_set_repetition_penalty` | Sets the multiplicative penalty applied to logits of tokens seen before.          |
| `litert_lm_repetition_penalty_config_set_presence_penalty`   | Sets a flat subtractive penalty for any token seen at least once.                 |
| `litert_lm_repetition_penalty_config_set_frequency_penalty`  | Sets a subtractive penalty scaled by how many times a token has appeared.         |
| `litert_lm_repetition_penalty_config_set_window_size`        | Sets how many recent tokens count toward these penalties (0 = unlimited history). |

### `NoRepeatNgramConfig`

| Function                                                    | What it does                                               |
| ----------------------------------------------------------- | ---------------------------------------------------------- |
| `litert_lm_no_repeat_ngram_config_create`                   | Creates a no-repeat-ngram config.                          |
| `litert_lm_no_repeat_ngram_config_delete`                   | Frees it.                                                  |
| `litert_lm_no_repeat_ngram_config_set_no_repeat_ngram_size` | Sets the ngram length that can't repeat during generation. |
| `litert_lm_no_repeat_ngram_config_set_window_size`          | Sets how many recent tokens are checked for repeats.       |

### `SuppressTokensConfig`

| Function                                               | What it does                                                    |
| ------------------------------------------------------ | --------------------------------------------------------------- |
| `litert_lm_suppress_tokens_config_create`              | Creates a suppress-tokens config.                               |
| `litert_lm_suppress_tokens_config_delete`              | Frees it.                                                       |
| `litert_lm_suppress_tokens_config_set_suppress_tokens` | Sets the list of token ids that are forced to never be sampled. |

### `ThinkingConfig`

| Function                                              | What it does                                                               |
| ----------------------------------------------------- | -------------------------------------------------------------------------- |
| `litert_lm_thinking_config_create`                    | Creates a thinking/reasoning config (enabled, infinite budget by default). |
| `litert_lm_thinking_config_delete`                    | Frees it.                                                                  |
| `litert_lm_thinking_config_set_enable_thinking`       | Turns reasoning generation on or off.                                      |
| `litert_lm_thinking_config_set_thinking_token_budget` | Caps how many tokens can be spent thinking (-1 = infinite).                |

### `SessionConfig`

| Function                                             | What it does                                         |
| ---------------------------------------------------- | ---------------------------------------------------- |
| `litert_lm_session_config_create`                    | Creates a session config.                            |
| `litert_lm_session_config_delete`                    | Frees it.                                            |
| `litert_lm_session_config_set_max_output_tokens`     | Caps output tokens per decode step.                  |
| `litert_lm_session_config_set_apply_prompt_template` | Turns automatic prompt-template rendering on or off. |
| `litert_lm_session_config_set_sampler_params`        | Attaches a `SamplerParams` to this session.          |
| `litert_lm_session_config_set_lora_path`             | Sets the path to a text LoRA weights file.           |
| `litert_lm_session_config_set_audio_lora_path`       | Sets the path to an audio LoRA weights file.         |

### `ConversationConfig`

| Function                                                                 | What it does                                               |
| ------------------------------------------------------------------------ | ---------------------------------------------------------- |
| `litert_lm_conversation_config_create`                                   | Creates a conversation config.                             |
| `litert_lm_conversation_config_delete`                                   | Frees it.                                                  |
| `litert_lm_conversation_config_set_session_config`                       | Attaches a `SessionConfig` to this conversation.           |
| `litert_lm_conversation_config_set_system_message`                       | Sets the system message (JSON).                            |
| `litert_lm_conversation_config_set_tools`                                | Sets the available tools (JSON array) for tool calling.    |
| `litert_lm_conversation_config_set_messages`                             | Seeds the conversation with initial messages (JSON array). |
| `litert_lm_conversation_config_set_extra_context`                        | Sets extra context injected into the conversation preface. |
| `litert_lm_conversation_config_set_prompt_template`                      | Overrides the model/engine's default prompt template.      |
| `litert_lm_conversation_config_set_enable_constrained_decoding`          | Turns constrained decoding on or off.                      |
| `litert_lm_conversation_config_set_constraint_provider`                  | Chooses the constraint provider backend (e.g. LlGuidance). |
| `litert_lm_conversation_config_set_filter_channel_content_from_kv_cache` | Toggles filtering channel content out of the KV cache.     |
| `litert_lm_conversation_config_set_stream_tool_calls`                    | Toggles streaming tool-call tokens on a named channel.     |
| `litert_lm_conversation_config_set_thinking_config`                      | Attaches a `ThinkingConfig`.                               |

### `ConversationOptionalArgs` (per-turn overrides)

| Function                                                             | What it does                                                     |
| -------------------------------------------------------------------- | ---------------------------------------------------------------- |
| `litert_lm_conversation_optional_args_create`                        | Creates a per-turn optional-args object.                         |
| `litert_lm_conversation_optional_args_delete`                        | Frees it.                                                        |
| `litert_lm_conversation_optional_args_set_repetition_penalty_config` | Applies a repetition-penalty config to just this turn.           |
| `litert_lm_conversation_optional_args_set_no_repeat_ngram_config`    | Applies a no-repeat-ngram config to just this turn.              |
| `litert_lm_conversation_optional_args_set_suppress_tokens_config`    | Applies a suppress-tokens config to just this turn.              |
| `litert_lm_conversation_optional_args_set_visual_token_budget`       | Caps vision tokens for just this turn.                           |
| `litert_lm_conversation_optional_args_set_max_output_tokens`         | Caps output tokens for just this turn.                           |
| `litert_lm_conversation_optional_args_set_thinking_config`           | Applies a thinking config to just this turn.                     |
| `litert_lm_conversation_optional_args_set_constraint`                | Sets a regex/JSON-schema constraint for just this turn's output. |

### `InputData`

| Function                      | What it does                                                                    |
| ----------------------------- | ------------------------------------------------------------------------------- |
| `litert_lm_input_data_create` | Creates a multimodal input chunk (text, image, image-end, audio, or audio-end). |
| `litert_lm_input_data_delete` | Frees it.                                                                       |

### `EngineSettings`

| Function                                                        | What it does                                                                   |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `litert_lm_engine_settings_create`                              | Creates engine settings from a model file path + backend names.                |
| `litert_lm_engine_settings_create_from_raw_file_descriptor`     | Same, but from an already-open file descriptor (engine takes ownership of it). |
| `litert_lm_engine_settings_delete`                              | Frees the settings.                                                            |
| `litert_lm_engine_settings_set_max_num_tokens`                  | Caps total tokens (context length).                                            |
| `litert_lm_engine_settings_set_num_threads`                     | Sets CPU backend thread count.                                                 |
| `litert_lm_engine_settings_set_audio_num_threads`               | Sets audio CPU backend thread count.                                           |
| `litert_lm_engine_settings_set_parallel_file_section_loading`   | Toggles loading `.litertlm` file sections in parallel (default on).            |
| `litert_lm_engine_settings_set_max_num_images`                  | Caps images for the legacy engine implementation.                              |
| `litert_lm_engine_settings_set_cache_dir`                       | Sets the on-disk cache directory.                                              |
| `litert_lm_engine_settings_set_litert_dispatch_lib_dir`         | Sets the NPU dispatch library directory.                                       |
| `litert_lm_engine_settings_set_activation_data_type`            | Chooses the activation dtype (fp32/fp16/int16/int8).                           |
| `litert_lm_engine_settings_set_prefill_chunk_size`              | Sets prefill chunk size (CPU backend, dynamic models only).                    |
| `litert_lm_engine_settings_enable_benchmark`                    | Turns on benchmark data collection.                                            |
| `litert_lm_engine_settings_set_num_prefill_tokens`              | Sets prefill token count used for benchmarking.                                |
| `litert_lm_engine_settings_set_num_decode_tokens`               | Sets decode token count used for benchmarking.                                 |
| `litert_lm_engine_settings_set_enable_speculative_decoding`     | Turns speculative decoding on or off.                                          |
| `litert_lm_engine_settings_set_gpu_decode_steps_per_sync`       | Sets decode steps per GPU sync (Artisan GPU backend only).                     |
| `litert_lm_engine_settings_set_gpu_wait_for_weight_uploads`     | Toggles waiting for GPU weight uploads (Artisan GPU backend only).             |
| `litert_lm_engine_settings_set_use_ringbuffers_local_attention` | Toggles ringbuffer KV cache for local attention (GPU Artisan only).            |
| `litert_lm_engine_settings_set_lora_rank`                       | Sets the (text) LoRA rank.                                                     |
| `litert_lm_engine_settings_set_supported_lora_ranks`            | Sets the list of supported (text) LoRA ranks.                                  |
| `litert_lm_engine_settings_set_audio_lora_rank`                 | Sets the audio LoRA rank.                                                      |
| `litert_lm_engine_settings_set_supported_audio_lora_ranks`      | Sets the list of supported audio LoRA ranks.                                   |

### `Engine`

| Function                           | What it does                                                        |
| ---------------------------------- | ------------------------------------------------------------------- |
| `litert_lm_engine_create`          | Creates the engine (loads the model) from settings.                 |
| `litert_lm_engine_delete`          | Frees the engine.                                                   |
| `litert_lm_engine_create_session`  | Creates a low-level `Session` for manual prefill/decode control.    |
| `litert_lm_conversation_create`    | Creates a `Conversation` (higher-level, chat-message-oriented API). |
| `litert_lm_engine_tokenize`        | Tokenizes a UTF-8 string using the model's tokenizer.               |
| `litert_lm_engine_detokenize`      | Converts token ids back into text.                                  |
| `litert_lm_engine_get_start_token` | Returns the configured BOS (start) token, if any.                   |
| `litert_lm_engine_get_stop_tokens` | Returns the configured EOS (stop) tokens, if any.                   |

### `Session` (low-level prefill/decode API)

| Function                                    | What it does                                                            |
| ------------------------------------------- | ----------------------------------------------------------------------- |
| `litert_lm_session_delete`                  | Frees the session.                                                      |
| `litert_lm_session_cancel_process`          | Cancels in-flight processing on this session.                           |
| `litert_lm_session_run_prefill`             | Blocking: feeds multimodal input into the model for prefill.            |
| `litert_lm_session_run_decode`              | Blocking: decodes a response after prefill.                             |
| `litert_lm_session_run_text_scoring`        | Blocking: scores given target texts against the prefilled context.      |
| `litert_lm_session_generate_content`        | Blocking: prefill + decode in one call.                                 |
| `litert_lm_session_run_decode_async`        | Streaming: decodes a response, invoking a callback per chunk.           |
| `litert_lm_session_generate_content_stream` | Streaming: prefill + decode in one call, invoking a callback per chunk. |
| `litert_lm_session_get_benchmark_info`      | Retrieves benchmark data collected on this session.                     |

### `Responses`

| Function                                      | What it does                                                |
| --------------------------------------------- | ----------------------------------------------------------- |
| `litert_lm_responses_delete`                  | Frees a responses object.                                   |
| `litert_lm_responses_get_num_candidates`      | Returns how many response candidates were generated.        |
| `litert_lm_responses_get_response_text_at`    | Returns the text of the candidate at an index.              |
| `litert_lm_responses_has_score_at`            | Whether a score is present for a candidate.                 |
| `litert_lm_responses_get_score_at`            | Returns the score for a candidate (e.g. from text scoring). |
| `litert_lm_responses_has_token_length_at`     | Whether a token length is present for a candidate.          |
| `litert_lm_responses_get_token_length_at`     | Returns the token length for a candidate.                   |
| `litert_lm_responses_has_token_scores_at`     | Whether per-token scores are present for a candidate.       |
| `litert_lm_responses_get_num_token_scores_at` | Returns how many per-token scores are present.              |
| `litert_lm_responses_get_token_scores_at`     | Returns the per-token scores array for a candidate.         |

### `BenchmarkInfo`

| Function                                                 | What it does                                       |
| -------------------------------------------------------- | -------------------------------------------------- |
| `litert_lm_benchmark_info_delete`                        | Frees a benchmark info object.                     |
| `litert_lm_benchmark_info_get_time_to_first_token`       | Seconds from prefill start to first decoded token. |
| `litert_lm_benchmark_info_get_total_init_time_in_second` | Total engine/model initialization time.            |
| `litert_lm_benchmark_info_get_num_prefill_turns`         | Number of prefill turns recorded.                  |
| `litert_lm_benchmark_info_get_num_decode_turns`          | Number of decode turns recorded.                   |
| `litert_lm_benchmark_info_get_prefill_token_count_at`    | Prefill token count for a given turn.              |
| `litert_lm_benchmark_info_get_decode_token_count_at`     | Decode token count for a given turn.               |
| `litert_lm_benchmark_info_get_prefill_tokens_per_sec_at` | Prefill throughput (tokens/sec) for a given turn.  |
| `litert_lm_benchmark_info_get_decode_tokens_per_sec_at`  | Decode throughput (tokens/sec) for a given turn.   |

### `Conversation` (high-level chat API)

| Function                                          | What it does                                                                           |
| ------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `litert_lm_conversation_delete`                   | Frees the conversation.                                                                |
| `litert_lm_conversation_clone`                    | Duplicates a conversation, including its prefilled state.                              |
| `litert_lm_conversation_send_message`             | Blocking: sends a message JSON, returns the full JSON response.                        |
| `litert_lm_conversation_send_message_stream`      | Streaming: sends a message JSON, invoking a callback per chunk.                        |
| `litert_lm_json_response_get_string`              | Extracts the JSON string from a response object.                                       |
| `litert_lm_json_response_delete`                  | Frees a response object.                                                               |
| `litert_lm_conversation_render_message_to_string` | Renders a message JSON through the prompt template without sending it.                 |
| `litert_lm_conversation_render_preface_to_string` | Renders the conversation's preface (system message, tools, etc.) through the template. |
| `litert_lm_conversation_cancel_process`           | Cancels in-flight processing on this conversation.                                     |
| `litert_lm_conversation_get_benchmark_info`       | Retrieves benchmark data collected on this conversation.                               |
| `litert_lm_conversation_get_token_count`          | Returns tokens currently held in the conversation's KV cache.                          |

### Streaming chunks

| Function                           | What it does                                           |
| ---------------------------------- | ------------------------------------------------------ |
| `litert_lm_stream_chunk_get_text`  | Returns the text content of a streamed chunk, if any.  |
| `litert_lm_stream_chunk_is_final`  | Whether this is the last chunk of the stream.          |
| `litert_lm_stream_chunk_get_error` | Returns the error message attached to a chunk, if any. |

### Tokenization results

| Function                                   | What it does                            |
| ------------------------------------------ | --------------------------------------- |
| `litert_lm_tokenize_result_delete`         | Frees a tokenize result.                |
| `litert_lm_tokenize_result_get_tokens`     | Returns the token ids array.            |
| `litert_lm_tokenize_result_get_num_tokens` | Returns how many token ids are present. |
| `litert_lm_detokenize_result_delete`       | Frees a detokenize result.              |
| `litert_lm_detokenize_result_get_string`   | Returns the detokenized text.           |

### Token unions (BOS/EOS representation)

| Function                                | What it does                                                  |
| --------------------------------------- | ------------------------------------------------------------- |
| `litert_lm_token_union_delete`          | Frees a token union.                                          |
| `litert_lm_token_union_get_type`        | Whether this token union holds a string or a sequence of ids. |
| `litert_lm_token_union_get_string`      | Returns the string value, if this union is a string.          |
| `litert_lm_token_union_get_ids`         | Returns the token id sequence, if this union is ids.          |
| `litert_lm_token_unions_delete`         | Frees a collection of token unions.                           |
| `litert_lm_token_unions_get_num_tokens` | Returns how many token unions are in the collection.          |
| `litert_lm_token_unions_get_token_at`   | Returns the token union at an index.                          |

## License

Apache-2.0.

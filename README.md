# litert-lm-rs

Rust bindings for Google's LiteRT LM C API (`conversation.h` + `engine.h`),
split into two crates:

- **[`litertlm-sys`](litertlm-sys)** — raw, unsafe `bindgen`-generated FFI bindings.
- **[`litertlm-rs`](litertlm-rs)** — a safe wrapper covering the core flow:
  `EngineSettings -> Engine -> Conversation -> send_message[_stream]`.

Links against Google's official C API prebuilt package (LiteRT-LM v0.16.0+),
a single self-contained shared library per platform — no Bazel build
required, no separate plugin libraries to hunt down. (If you're on an older
LiteRT-LM version without this prebuilt, see "Building from source instead"
near the bottom of this README for the old, much more involved path this
project used to require.)

## Setup

1. **System requirement:** `bindgen` needs `libclang` installed to run at
   build time (developer-only dependency, not needed by compiled binary):
   ```bash
   # Debian/Ubuntu
   sudo apt install libclang-dev clang
   # Fedora
   sudo dnf install clang-devel
   # macOS
   brew install llvm
   ```
2. `litertlm-sys` will download the needed C API prebuilt library
   automatically (from an independent prebuilt mirror to avoid wasteful
   bulk downloads). If you'd rather supply your own downloaded copy instead,
   go to step 3.

3. **Download the official C API prebuilt package** for your platform from
   [LiteRT-LM's GitHub releases](https://github.com/google-ai-edge/LiteRT-LM/releases)
   (v0.16.0 or later — look for the C API prebuilt asset, not the source
   tarball). It contains:

   ```
   include/{conversation.h, engine.h}
   lib/<platform>/liblitert-lm.so   (or .dylib, or litert-lm.dll + litert-lm.lib on Windows)
   ```

4. **Extract it into `your/project/root/prebuilt/`**, so the layout matches:

   ```
   your/project/root/
   ├── .cargo
   │   └── config.toml
   ├── Cargo.toml
   ├── src/
   └── prebuilt/                      (or any other name, e.g. "native")
       └── lib/
           └── linux_x86_64/          (or your platform)
               └── liblitert-lm.so
   ```

   The headers are already committed in this repo's own `litertlm-sys/prebuilt/`;
   for your own project you only need the `lib/` directory from your
   download. See [`litertlm-sys/prebuilt/README.md`](litertlm-sys/prebuilt/README.md)
   for the full expected layout across all platforms.

   Add these lines to your project's `.cargo/config.toml` so `litertlm-sys`
   knows where to find it:

   ```toml
   [env]
   LITERT_LM_LIB_DIR = { value = "prebuilt", relative = true }
   ```

   or

   ```sh
   LITERT_LM_LIB_DIR="path/to/prebuilt/" cargo build
   ```

5. Build:

   ```bash
   cargo build
   ```

6. Run an example (needs a real model file):
   ```bash
   cargo run --example basic -- /path/to/model.litertlm [gpu/cpu]
   cargo run --example streaming -- /path/to/model.litertlm [gpu/cpu]
   cargo run --example interactive -- /path/to/model.litertlm cpu
   cargo run --example interactive -- /path/to/model.litertlm gpu
   ```

## Cross-compiling to Windows

The official LiteRT-LM Windows prebuilt only ships an MSVC-format
import library (`litert-lm.lib`), so this crate only supports the
**MSVC** target — `x86_64-pc-windows-gnu` (MinGW) isn't supported,
since GNU `ld` can't link against an MSVC-format `.lib` directly.

**On Windows:** just use the MSVC target, which is the default toolchain
most Windows Rust installs already have:

```bash
rustup target add x86_64-pc-windows-msvc
cargo build --target x86_64-pc-windows-msvc
```

**Cross-compiling from Linux or macOS:** use [`cargo-xwin`](https://github.com/rust-cross/cargo-xwin),
which downloads a minimal Windows SDK/CRT so you can target MSVC
without a real Windows machine:

```bash
cargo install cargo-xwin
rustup target add x86_64-pc-windows-msvc
cargo xwin build --target x86_64-pc-windows-msvc
```

## Runtime linking

`litertlm-sys/build.rs` locates (or downloads) the native library and tells
cargo where it is, but `rustc-link-arg` — which is what embeds the rpath —
only applies to build targets in the _same package_ as the script that
emits it, and `litertlm-sys` doesn't build any binaries itself. So
`litertlm-sys` instead passes the library's location to `litertlm-rs`'s
build script via Cargo's `links` metadata mechanism
(`DEP_LITERT_LM_LIB_DIR` / `DEP_LITERT_LM_LIB_FILENAME`), and it's
`litertlm-rs/build.rs` that actually copies the library next to your
compiled binaries and embeds the rpath, so `liblitert-lm.so` (or the
`.dylib`/`.dll`) is found automatically at runtime without
`LD_LIBRARY_PATH`.

Because this is now a single self-contained library (unlike the old
locally-built `libengine.so`, which depended on several sibling plugin
`.so` files), the modern default rpath tag (`RUNPATH`, which only resolves
an object's own _direct_ dependencies) is sufficient on its own — no more
`--disable-new-dtags`, and no more `patchelf` step needed at all.

`litertlm-rs/build.rs` auto-bundles: every build copies the native library
flat next to wherever cargo might place a binary (`target/<profile>/`,
`target/<profile>/examples/`, `target/<profile>/deps/`), and embeds an
`$ORIGIN`-relative rpath (`@loader_path` on macOS) pointing at that same
directory, plus a fallback rpath pointing straight at the build-time
location. So `cargo build --release` alone produces a self-contained
`target/release/` — your binary plus `liblitert-lm.so`/`.dylib`/`.dll`
sitting right next to it — that you can zip up and run on another machine
without needing this whole checkout present:

```bash
cargo build --release
cd target/release && zip myapp.zip your-binary liblitert-lm.so   # adjust for your platform
```

**Windows**: covered natively by the official prebuilt (`litert-lm.dll` +
`litert-lm.lib` import library) — this is actually the one part of the
whole setup story that's _easier_ now than it would have been building
from source, since a matching `.lib` import library is guaranteed to exist.
No rpath concept applies there; the Windows loader finds `litert-lm.dll`
because `build.rs` copies it next to the `.exe`.

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

## Building from source instead

If you need a LiteRT-LM version that doesn't have an official C API
prebuilt yet, or need to build from an unreleased commit, the old path
this project used before v0.16.0 still works — it's just considerably more
involved: Bazel doesn't expose a redistributable shared library from the
plain `//c:engine` target (only a `cc_library`), so you need a
`linkshared = True, linkstatic = True` wrapper target, then `patchelf` each
resulting `.so` (plus Google's separately-shipped GPU/constraint-provider
plugin `.so`s from `prebuilt/<platform>/` in that older sense — a
LiteRT-LM-repo-internal `prebuilt/` directory, not to be confused with the
official _C API_ prebuilt this README now centers on) to resolve each
other correctly at runtime, and force the `bfd` linker since `rust-lld`
rejects the resulting `.so`'s symbol-version table. See this README's git
history for the full blow-by-blow if you need it — it's a substantially
longer and more fragile process than the setup above, which is exactly why
switching to the official prebuilt was worth doing.

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

---

# litertlm-rs Usage Guide

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

- [`litertlm-rs/examples/basic.rs`](litertlm-rs/examples/basic.rs) — Hello world
- [`litertlm-rs/examples/streaming.rs`](litertlm-rs/examples/streaming.rs) — Streaming output
- [`litertlm-rs/examples/interactive.rs`](litertlm-rs/examples/interactive.rs) — Interactive chat REPL with thinking
- [`litertlm-rs/examples/sandbox_tools.rs`](litertlm-rs/examples/sandbox_tools.rs) — Tool calling with sandboxed file access
- [`litertlm-rs/examples/more_configs`](litertlm-rs/examples/more_configs.rs) — configuration with thinking, sampler, and constraints

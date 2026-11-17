//! Engine settings, [`Engine`] and [`Session`].
//!
//! # Lifetime safety
//!
//! The C API requires every session and conversation created from an engine
//! to be destroyed **before** the engine (anything else is undefined
//! behavior). This crate makes that impossible to get wrong: [`Session`]
//! and [`Conversation`](crate::Conversation) each hold a reference-counted
//! handle to the engine, so the underlying engine is destroyed only after
//! the last of them is dropped, in whatever order you drop them.

use std::cell::Cell;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Arc;

use litertlm_sys as sys;

use crate::config::*;
use crate::error::{Error, Result};
use crate::results::*;
use crate::stream::{run_stream, StreamChunk};
use crate::types::*;
use crate::util::*;
use crate::version::check_api_version;

// =======================================================================
// EngineSettings
// =======================================================================

/// Builder for `LiteRtLmEngineSettings`.
pub struct EngineSettings {
    ptr: NonNull<sys::LiteRtLmEngineSettings>,
    single_threaded: bool,
}

// SAFETY: see `owned_handle!`.
unsafe impl Send for EngineSettings {}

impl std::fmt::Debug for EngineSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineSettings").finish_non_exhaustive()
    }
}

impl EngineSettings {
    /// Wraps `litert_lm_engine_settings_create`. `backend`: e.g. `"cpu"` or
    /// `"gpu"`. `vision_backend` / `audio_backend` may be left `None`.
    ///
    /// Also verifies (once per process) that the loaded `liblitert-lm` is
    /// ABI-compatible with this crate; see [`check_api_version`].
    pub fn new(
        model_path: &str,
        backend: &str,
        vision_backend: Option<&str>,
        audio_backend: Option<&str>,
    ) -> Result<Self> {
        check_api_version()?;
        let model_path = cstr("model_path", model_path)?;
        let backend = cstr("backend", backend)?;
        let vision_backend = opt_cstr("vision_backend", vision_backend)?;
        let audio_backend = opt_cstr("audio_backend", audio_backend)?;

        let ptr = create("litert_lm_engine_settings_create", |out| unsafe {
            sys::litert_lm_engine_settings_create(
                model_path.as_ptr(),
                backend.as_ptr(),
                opt_ptr(&vision_backend),
                opt_ptr(&audio_backend),
                out,
            )
        })?;
        Ok(Self {
            ptr,
            single_threaded: false,
        })
    }

    /// Wraps `litert_lm_engine_settings_create_from_raw_file_descriptor`.
    /// The engine takes ownership of `fd` and will close it when done.
    pub fn from_raw_file_descriptor(
        fd: i32,
        backend: &str,
        vision_backend: Option<&str>,
        audio_backend: Option<&str>,
    ) -> Result<Self> {
        check_api_version()?;
        let backend = cstr("backend", backend)?;
        let vision_backend = opt_cstr("vision_backend", vision_backend)?;
        let audio_backend = opt_cstr("audio_backend", audio_backend)?;

        let ptr = create(
            "litert_lm_engine_settings_create_from_raw_file_descriptor",
            |out| unsafe {
                sys::litert_lm_engine_settings_create_from_raw_file_descriptor(
                    fd,
                    backend.as_ptr(),
                    opt_ptr(&vision_backend),
                    opt_ptr(&audio_backend),
                    out,
                )
            },
        )?;
        Ok(Self {
            ptr,
            single_threaded: false,
        })
    }

    fn raw(&self) -> *mut sys::LiteRtLmEngineSettings {
        self.ptr.as_ptr()
    }

    /// Wraps `litert_lm_engine_settings_set_max_num_tokens`.
    pub fn set_max_num_tokens(&mut self, max_num_tokens: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_max_num_tokens(self.raw(), max_num_tokens)
            },
            "litert_lm_engine_settings_set_max_num_tokens",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_num_threads`. No effect unless
    /// the main backend is CPU.
    pub fn set_num_threads(&mut self, num_threads: i32) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_engine_settings_set_num_threads(self.raw(), num_threads) },
            "litert_lm_engine_settings_set_num_threads",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_audio_num_threads`.
    pub fn set_audio_num_threads(&mut self, num_threads: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_audio_num_threads(self.raw(), num_threads)
            },
            "litert_lm_engine_settings_set_audio_num_threads",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_parallel_file_section_loading`.
    /// Defaults to `true`.
    pub fn set_parallel_file_section_loading(&mut self, parallel: bool) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_parallel_file_section_loading(
                    self.raw(),
                    parallel,
                )
            },
            "litert_lm_engine_settings_set_parallel_file_section_loading",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_single_threaded_execution`
    /// (new in C API 1.0.0).
    ///
    /// When enabled, the engine does no background work of its own: queued
    /// work (and with it stream callbacks) only runs while a caller drives
    /// it, and the "no overlapping calls" rule extends to the engine and
    /// every session and conversation created from it. This crate drives
    /// conversation streams for you (via `wait_until_done`); **session
    /// streaming is not supported in this mode** and returns
    /// [`Error::Unsupported`].
    pub fn set_single_threaded_execution(&mut self, single_threaded: bool) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_single_threaded_execution(
                    self.raw(),
                    single_threaded,
                )
            },
            "litert_lm_engine_settings_set_single_threaded_execution",
        );
        self.single_threaded = single_threaded;
        self
    }

    /// Wraps `litert_lm_engine_settings_set_max_num_images`. Only used by
    /// the legacy engine implementation.
    pub fn set_max_num_images(&mut self, max_num_images: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_max_num_images(self.raw(), max_num_images)
            },
            "litert_lm_engine_settings_set_max_num_images",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_max_vision_tokens_per_image`
    /// (new in C API 1.0.0).
    pub fn set_max_vision_tokens_per_image(&mut self, max_vision_tokens: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_max_vision_tokens_per_image(
                    self.raw(),
                    max_vision_tokens,
                )
            },
            "litert_lm_engine_settings_set_max_vision_tokens_per_image",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_cache_dir`.
    pub fn set_cache_dir(&mut self, cache_dir: &str) -> Result<&mut Self> {
        let cache_dir = cstr("cache_dir", cache_dir)?;
        check(
            unsafe { sys::litert_lm_engine_settings_set_cache_dir(self.raw(), cache_dir.as_ptr()) },
            "litert_lm_engine_settings_set_cache_dir",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_engine_settings_set_litert_dispatch_lib_dir`.
    pub fn set_litert_dispatch_lib_dir(&mut self, lib_dir: &str) -> Result<&mut Self> {
        let lib_dir = cstr("lib_dir", lib_dir)?;
        check(
            unsafe {
                sys::litert_lm_engine_settings_set_litert_dispatch_lib_dir(
                    self.raw(),
                    lib_dir.as_ptr(),
                )
            },
            "litert_lm_engine_settings_set_litert_dispatch_lib_dir",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_engine_settings_set_activation_data_type`.
    pub fn set_activation_data_type(&mut self, data_type: ActivationDataType) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_activation_data_type(
                    self.raw(),
                    data_type.into(),
                )
            },
            "litert_lm_engine_settings_set_activation_data_type",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_prefill_chunk_size`. Only
    /// applicable for the CPU backend with dynamic models.
    pub fn set_prefill_chunk_size(&mut self, chunk_size: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_prefill_chunk_size(self.raw(), chunk_size)
            },
            "litert_lm_engine_settings_set_prefill_chunk_size",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_enable_ynnpack`. Controls
    /// whether YNNPACK is allowed to delegate supported operations ahead of
    /// XNNPACK (CPU backend only).
    pub fn set_enable_ynnpack(&mut self, enable: bool) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_engine_settings_set_enable_ynnpack(self.raw(), enable) },
            "litert_lm_engine_settings_set_enable_ynnpack",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_enable_benchmark`.
    pub fn enable_benchmark(&mut self) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_engine_settings_enable_benchmark(self.raw()) },
            "litert_lm_engine_settings_enable_benchmark",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_num_prefill_tokens`.
    pub fn set_num_prefill_tokens(&mut self, num_prefill_tokens: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_num_prefill_tokens(self.raw(), num_prefill_tokens)
            },
            "litert_lm_engine_settings_set_num_prefill_tokens",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_num_decode_tokens`.
    pub fn set_num_decode_tokens(&mut self, num_decode_tokens: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_num_decode_tokens(self.raw(), num_decode_tokens)
            },
            "litert_lm_engine_settings_set_num_decode_tokens",
        );
        self
    }

    /// Wraps the deprecated `litert_lm_engine_settings_set_enable_speculative_decoding`.
    #[deprecated(note = "use `set_speculative_decoding_config` instead")]
    pub fn set_enable_speculative_decoding(&mut self, enable: bool) -> &mut Self {
        #[allow(deprecated)]
        let status = unsafe {
            sys::litert_lm_engine_settings_set_enable_speculative_decoding(self.raw(), enable)
        };
        infallible(
            status,
            "litert_lm_engine_settings_set_enable_speculative_decoding",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_speculative_decoding_config`.
    pub fn set_speculative_decoding_config(
        &mut self,
        config: &SpeculativeDecodingConfig,
    ) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_speculative_decoding_config(
                    self.raw(),
                    config.as_ptr(),
                )
            },
            "litert_lm_engine_settings_set_speculative_decoding_config",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_gpu_decode_steps_per_sync`.
    /// Only supported on the Artisan GPU backend.
    pub fn set_gpu_decode_steps_per_sync(&mut self, steps: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_gpu_decode_steps_per_sync(self.raw(), steps)
            },
            "litert_lm_engine_settings_set_gpu_decode_steps_per_sync",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_gpu_wait_for_weight_uploads`.
    /// Only supported on the Artisan GPU backend.
    pub fn set_gpu_wait_for_weight_uploads(&mut self, wait: bool) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_gpu_wait_for_weight_uploads(self.raw(), wait)
            },
            "litert_lm_engine_settings_set_gpu_wait_for_weight_uploads",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_use_ringbuffers_local_attention`.
    /// Currently only honored on the GPU Artisan backend.
    pub fn set_use_ringbuffers_local_attention(&mut self, use_ringbuffers: bool) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_use_ringbuffers_local_attention(
                    self.raw(),
                    use_ringbuffers,
                )
            },
            "litert_lm_engine_settings_set_use_ringbuffers_local_attention",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_gpu_enable_metal_residency_set`
    /// (new in C API 1.0.0). Recorded on all platforms, effective only on
    /// Metal GPU backends.
    pub fn set_gpu_enable_metal_residency_set(&mut self, enable: bool) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_engine_settings_set_gpu_enable_metal_residency_set(
                    self.raw(),
                    enable,
                )
            },
            "litert_lm_engine_settings_set_gpu_enable_metal_residency_set",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_lora_rank`.
    pub fn set_lora_rank(&mut self, lora_rank: i32) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_engine_settings_set_lora_rank(self.raw(), lora_rank) },
            "litert_lm_engine_settings_set_lora_rank",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_supported_lora_ranks`. `ranks`
    /// must not be empty.
    pub fn set_supported_lora_ranks(&mut self, ranks: &[i32]) -> Result<&mut Self> {
        check(
            unsafe {
                sys::litert_lm_engine_settings_set_supported_lora_ranks(
                    self.raw(),
                    ranks.as_ptr(),
                    ranks.len(),
                )
            },
            "litert_lm_engine_settings_set_supported_lora_ranks",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_engine_settings_set_audio_lora_rank`.
    pub fn set_audio_lora_rank(&mut self, lora_rank: i32) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_engine_settings_set_audio_lora_rank(self.raw(), lora_rank) },
            "litert_lm_engine_settings_set_audio_lora_rank",
        );
        self
    }

    /// Wraps `litert_lm_engine_settings_set_supported_audio_lora_ranks`.
    /// `ranks` must not be empty, and an audio executor must be configured.
    pub fn set_supported_audio_lora_ranks(&mut self, ranks: &[i32]) -> Result<&mut Self> {
        check(
            unsafe {
                sys::litert_lm_engine_settings_set_supported_audio_lora_ranks(
                    self.raw(),
                    ranks.as_ptr(),
                    ranks.len(),
                )
            },
            "litert_lm_engine_settings_set_supported_audio_lora_ranks",
        )?;
        Ok(self)
    }
}

impl Drop for EngineSettings {
    fn drop(&mut self) {
        unsafe { sys::litert_lm_engine_settings_delete(self.ptr.as_ptr()) };
    }
}

// =======================================================================
// Engine
// =======================================================================

/// The reference-counted owner of the C engine handle. Sessions and
/// conversations keep one alive so the engine always outlives them.
pub(crate) struct EngineHandle {
    ptr: NonNull<sys::LiteRtLmEngine>,
    pub(crate) single_threaded: bool,
}

// SAFETY: `EngineHandle` is only ever *used* through `Engine`'s methods,
// and `Engine` is `!Sync`, so calls on the engine handle never overlap.
// Children only hold the `Arc` to keep the handle alive; the last owner,
// on whatever thread, deletes it.
unsafe impl Send for EngineHandle {}
unsafe impl Sync for EngineHandle {}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        // SAFETY: every child holds an `Arc<EngineHandle>`, so by the time
        // this runs all sessions/conversations are already destroyed, as
        // `litert_lm_engine_delete` requires.
        unsafe { sys::litert_lm_engine_delete(self.ptr.as_ptr()) };
    }
}

/// A LiteRT-LM engine: the loaded model plus shared runtime resources.
pub struct Engine {
    inner: Arc<EngineHandle>,
    // Not `Sync`: calls on one engine handle must not overlap.
    _not_sync: PhantomData<Cell<()>>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}

impl Engine {
    /// Wraps `litert_lm_engine_create`.
    pub fn new(settings: &EngineSettings) -> Result<Self> {
        let ptr = create("litert_lm_engine_create", |out| unsafe {
            sys::litert_lm_engine_create(settings.raw(), out)
        })?;
        Ok(Self {
            inner: Arc::new(EngineHandle {
                ptr,
                single_threaded: settings.single_threaded,
            }),
            _not_sync: PhantomData,
        })
    }

    pub(crate) fn as_ptr(&self) -> *mut sys::LiteRtLmEngine {
        self.inner.ptr.as_ptr()
    }

    pub(crate) fn handle(&self) -> Arc<EngineHandle> {
        self.inner.clone()
    }

    /// Wraps `litert_lm_engine_create_session`. Pass `None` to use the
    /// default session config.
    pub fn create_session(&self, config: Option<&SessionConfig>) -> Result<Session> {
        let config_ptr = config.map_or(std::ptr::null_mut(), |c| c.as_ptr());
        let ptr = create("litert_lm_engine_create_session", |out| unsafe {
            sys::litert_lm_engine_create_session(self.as_ptr(), config_ptr, out)
        })?;
        Ok(Session {
            ptr,
            cancelled: Cell::new(false),
            engine: self.handle(),
        })
    }

    /// Wraps `litert_lm_conversation_create` with the default conversation
    /// config. See [`Engine::create_conversation_with_config`] to customize
    /// system message, tools, thinking, etc.
    pub fn create_conversation(&self) -> Result<crate::Conversation> {
        crate::conversation::create_conversation(self, std::ptr::null_mut())
    }

    /// Wraps `litert_lm_conversation_create` with an explicit
    /// [`ConversationConfig`].
    pub fn create_conversation_with_config(
        &self,
        config: &ConversationConfig,
    ) -> Result<crate::Conversation> {
        crate::conversation::create_conversation(self, config.as_ptr())
    }

    /// Wraps `litert_lm_engine_tokenize`.
    pub fn tokenize(&self, text: &str) -> Result<TokenizeResult> {
        let text_c = cstr("text", text)?;
        let ptr = create("litert_lm_engine_tokenize", |out| unsafe {
            sys::litert_lm_engine_tokenize(self.as_ptr(), text_c.as_ptr(), out)
        })?;
        Ok(TokenizeResult::from_non_null(ptr))
    }

    /// Wraps `litert_lm_engine_detokenize`.
    pub fn detokenize(&self, tokens: &[i32]) -> Result<DetokenizeResult> {
        let ptr = create("litert_lm_engine_detokenize", |out| unsafe {
            sys::litert_lm_engine_detokenize(self.as_ptr(), tokens.as_ptr(), tokens.len(), out)
        })?;
        Ok(DetokenizeResult::from_non_null(ptr))
    }

    /// Wraps `litert_lm_engine_get_start_token`. `Ok(None)` if no start
    /// (BOS) token is configured.
    pub fn get_start_token(&self) -> Result<Option<TokenUnion>> {
        Ok(
            create_optional("litert_lm_engine_get_start_token", |out| unsafe {
                sys::litert_lm_engine_get_start_token(self.as_ptr(), out)
            })?
            .map(TokenUnion::from_non_null),
        )
    }

    /// Wraps `litert_lm_engine_get_stop_tokens`. `Ok(None)` if no stop
    /// (EOS) tokens are configured.
    pub fn get_stop_tokens(&self) -> Result<Option<TokenUnions>> {
        Ok(
            create_optional("litert_lm_engine_get_stop_tokens", |out| unsafe {
                sys::litert_lm_engine_get_stop_tokens(self.as_ptr(), out)
            })?
            .map(TokenUnions::from_non_null),
        )
    }
}

// =======================================================================
// Session
// =======================================================================

/// A low-level session: explicit prefill/decode control, checkpoints,
/// scoring and streaming. Prefer [`Conversation`](crate::Conversation) for
/// chat-style use.
///
/// Holds the engine alive (see [`Engine`]). Not `Sync`.
pub struct Session {
    ptr: NonNull<sys::LiteRtLmSession>,
    cancelled: Cell<bool>,
    // Declared last so the engine is released after the session is deleted.
    engine: Arc<EngineHandle>,
}

// SAFETY: see `owned_handle!`; the `Arc` is `Send`, `Cell<bool>` is `Send`.
unsafe impl Send for Session {}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session").finish_non_exhaustive()
    }
}

impl Session {
    pub(crate) fn as_ptr(&self) -> *mut sys::LiteRtLmSession {
        self.ptr.as_ptr()
    }

    /// The C API forbids reusing a session after `cancel_process`.
    fn usable(&self, what: &'static str) -> Result<()> {
        if self.cancelled.get() {
            Err(Error::UseAfterCancel(what))
        } else {
            Ok(())
        }
    }

    /// Wraps `litert_lm_session_clone` (new in C API 1.0.0): copies this
    /// session's state into an independent session.
    pub fn clone_session(&self) -> Result<Session> {
        self.usable("litert_lm_session_clone")?;
        let ptr = create("litert_lm_session_clone", |out| unsafe {
            sys::litert_lm_session_clone(self.as_ptr(), out)
        })?;
        Ok(Session {
            ptr,
            cancelled: Cell::new(false),
            engine: self.engine.clone(),
        })
    }

    /// Wraps `litert_lm_session_cancel_process`.
    ///
    /// Per the C API, a cancelled session **must only be dropped**: every
    /// further call on it returns [`Error::UseAfterCancel`]. May be called
    /// from inside a stream callback while handling a non-final chunk.
    pub fn cancel_process(&self) -> Result<()> {
        check(
            unsafe { sys::litert_lm_session_cancel_process(self.as_ptr()) },
            "litert_lm_session_cancel_process",
        )?;
        self.cancelled.set(true);
        Ok(())
    }

    /// Wraps `litert_lm_session_save_checkpoint`. Saves the session's
    /// current state under `label`, so you can later return to it with
    /// [`Session::rewind_to_checkpoint`].
    pub fn save_checkpoint(&self, label: &str) -> Result<()> {
        self.usable("litert_lm_session_save_checkpoint")?;
        let label_c = cstr("label", label)?;
        check(
            unsafe { sys::litert_lm_session_save_checkpoint(self.as_ptr(), label_c.as_ptr()) },
            "litert_lm_session_save_checkpoint",
        )
    }

    /// Wraps `litert_lm_session_rewind_to_checkpoint`.
    pub fn rewind_to_checkpoint(&self, label: &str) -> Result<()> {
        self.usable("litert_lm_session_rewind_to_checkpoint")?;
        let label_c = cstr("label", label)?;
        check(
            unsafe { sys::litert_lm_session_rewind_to_checkpoint(self.as_ptr(), label_c.as_ptr()) },
            "litert_lm_session_rewind_to_checkpoint",
        )
    }

    /// Wraps `litert_lm_session_rewind_to_step`.
    pub fn rewind_to_step(&self, step: i32) -> Result<()> {
        self.usable("litert_lm_session_rewind_to_step")?;
        check(
            unsafe { sys::litert_lm_session_rewind_to_step(self.as_ptr(), step) },
            "litert_lm_session_rewind_to_step",
        )
    }

    /// Wraps `litert_lm_session_run_prefill`. Blocks until prefill
    /// completes.
    pub fn run_prefill(&self, inputs: &[InputData]) -> Result<()> {
        self.usable("litert_lm_session_run_prefill")?;
        let ptrs = input_ptrs(inputs);
        check(
            unsafe {
                sys::litert_lm_session_run_prefill(self.as_ptr(), ptrs.as_ptr(), ptrs.len())
            },
            "litert_lm_session_run_prefill",
        )
    }

    /// Wraps `litert_lm_session_run_decode`. Blocks until decoding
    /// completes; call after [`Session::run_prefill`].
    pub fn run_decode(&self) -> Result<Responses> {
        self.usable("litert_lm_session_run_decode")?;
        let ptr = create("litert_lm_session_run_decode", |out| unsafe {
            sys::litert_lm_session_run_decode(self.as_ptr(), out)
        })?;
        Ok(Responses::from_non_null(ptr))
    }

    /// Wraps `litert_lm_session_run_text_scoring`. Scores the given target
    /// texts after prefill. `target_texts` must not be empty.
    pub fn run_text_scoring(
        &self,
        target_texts: &[&str],
        store_token_lengths: bool,
    ) -> Result<Responses> {
        self.usable("litert_lm_session_run_text_scoring")?;
        let c_strings = target_texts
            .iter()
            .map(|s| cstr("target_texts", s))
            .collect::<Result<Vec<_>>>()?;
        let mut ptrs: Vec<*const std::ffi::c_char> = c_strings.iter().map(|s| s.as_ptr()).collect();
        let ptr = create("litert_lm_session_run_text_scoring", |out| unsafe {
            sys::litert_lm_session_run_text_scoring(
                self.as_ptr(),
                ptrs.as_mut_ptr(),
                ptrs.len(),
                store_token_lengths,
                out,
            )
        })?;
        Ok(Responses::from_non_null(ptr))
    }

    /// Wraps `litert_lm_session_generate_content`. Combines prefill +
    /// decode into one blocking call.
    pub fn generate_content(&self, inputs: &[InputData]) -> Result<Responses> {
        self.usable("litert_lm_session_generate_content")?;
        let ptrs = input_ptrs(inputs);
        let ptr = create("litert_lm_session_generate_content", |out| unsafe {
            sys::litert_lm_session_generate_content(
                self.as_ptr(),
                ptrs.as_ptr(),
                ptrs.len(),
                out,
            )
        })?;
        Ok(Responses::from_non_null(ptr))
    }

    fn streaming_supported(&self, what: &'static str) -> Result<()> {
        if self.engine.single_threaded {
            Err(Error::Unsupported(match what {
                "litert_lm_session_run_decode_async" => {
                    "Session::run_decode_async with single-threaded execution \
                     (use a Conversation, or the blocking Session APIs)"
                }
                _ => {
                    "Session::generate_content_stream with single-threaded execution \
                     (use a Conversation, or the blocking Session APIs)"
                }
            }))
        } else {
            Ok(())
        }
    }

    /// Wraps `litert_lm_session_run_decode_async`, blocking internally
    /// until the stream completes (see the [`stream`](crate::stream) module
    /// docs for the callback rules). Call after [`Session::run_prefill`].
    pub fn run_decode_async<F: FnMut(&StreamChunk)>(&self, on_chunk: F) -> Result<()> {
        const WHAT: &str = "litert_lm_session_run_decode_async";
        self.usable(WHAT)?;
        self.streaming_supported(WHAT)?;
        run_stream(
            WHAT,
            on_chunk,
            |cb, data| unsafe { sys::litert_lm_session_run_decode_async(self.as_ptr(), cb, data) },
            || Ok(()),
        )
    }

    /// Wraps `litert_lm_session_generate_content_stream`, blocking
    /// internally until the stream completes (see the
    /// [`stream`](crate::stream) module docs for the callback rules).
    pub fn generate_content_stream<F: FnMut(&StreamChunk)>(
        &self,
        inputs: &[InputData],
        on_chunk: F,
    ) -> Result<()> {
        const WHAT: &str = "litert_lm_session_generate_content_stream";
        self.usable(WHAT)?;
        self.streaming_supported(WHAT)?;
        let ptrs = input_ptrs(inputs);
        run_stream(
            WHAT,
            on_chunk,
            |cb, data| unsafe {
                sys::litert_lm_session_generate_content_stream(
                    self.as_ptr(),
                    ptrs.as_ptr(),
                    ptrs.len(),
                    cb,
                    data,
                )
            },
            || Ok(()),
        )
    }

    /// Wraps `litert_lm_session_get_benchmark_info`. Requires
    /// [`EngineSettings::enable_benchmark`] to have been called before the
    /// engine was created.
    pub fn get_benchmark_info(&self) -> Result<BenchmarkInfo> {
        self.usable("litert_lm_session_get_benchmark_info")?;
        let ptr = create("litert_lm_session_get_benchmark_info", |out| unsafe {
            sys::litert_lm_session_get_benchmark_info(self.as_ptr(), out)
        })?;
        Ok(BenchmarkInfo::from_non_null(ptr))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { sys::litert_lm_session_delete(self.ptr.as_ptr()) };
    }
}

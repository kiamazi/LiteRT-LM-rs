//! Embedding engine (`embedding_engine.h`).
//!
//! ```no_run
//! use litertlm_rs::{EmbeddingEngine, EmbeddingEngineSettings, InputData};
//!
//! # fn main() -> litertlm_rs::Result<()> {
//! let settings = EmbeddingEngineSettings::new("embedder.litertlm", "cpu", None, None)?;
//! let engine = EmbeddingEngine::new(&settings)?;
//! let response = engine.compute_embedding(&[InputData::text("hello")?], None)?;
//! let vector: &[f32] = response.values()?;
//! # let _ = vector;
//! # Ok(())
//! # }
//! ```

use std::marker::PhantomData;

use litertlm_sys as sys;

use crate::config::{input_ptrs, InputData};
use crate::error::{Error, Result};
use crate::types::*;
use crate::util::*;
use crate::version::check_api_version;

// =======================================================================
// EmbeddingEngineSettings
// =======================================================================

owned_handle!(
    /// Settings for an [`EmbeddingEngine`]. Wraps
    /// `LiteRtLmEmbeddingEngineSettings`.
    EmbeddingEngineSettings,
    LiteRtLmEmbeddingEngineSettings,
    litert_lm_embedding_engine_settings_delete
);

impl EmbeddingEngineSettings {
    /// Wraps `litert_lm_embedding_engine_settings_create`. `backend`: e.g.
    /// `"cpu"` or `"gpu"`; `vision_backend` / `audio_backend` may be `None`.
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
        let ptr = create("litert_lm_embedding_engine_settings_create", |out| unsafe {
            sys::litert_lm_embedding_engine_settings_create(
                model_path.as_ptr(),
                backend.as_ptr(),
                opt_ptr(&vision_backend),
                opt_ptr(&audio_backend),
                out,
            )
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_embedding_engine_settings_set_num_threads`. A
    /// non-positive value has no effect.
    pub fn set_num_threads(&mut self, num_threads: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_num_threads(
                    self.as_ptr(),
                    num_threads,
                )
            },
            "litert_lm_embedding_engine_settings_set_num_threads",
        );
        self
    }

    /// Wraps `litert_lm_embedding_engine_settings_set_audio_num_threads`.
    pub fn set_audio_num_threads(&mut self, num_threads: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_audio_num_threads(
                    self.as_ptr(),
                    num_threads,
                )
            },
            "litert_lm_embedding_engine_settings_set_audio_num_threads",
        );
        self
    }

    /// Wraps `litert_lm_embedding_engine_settings_set_cache_dir`.
    pub fn set_cache_dir(&mut self, cache_dir: &str) -> Result<&mut Self> {
        let cache_dir = cstr("cache_dir", cache_dir)?;
        check(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_cache_dir(
                    self.as_ptr(),
                    cache_dir.as_ptr(),
                )
            },
            "litert_lm_embedding_engine_settings_set_cache_dir",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_embedding_engine_settings_set_litert_dispatch_lib_dir`.
    pub fn set_litert_dispatch_lib_dir(&mut self, lib_dir: &str) -> Result<&mut Self> {
        let lib_dir = cstr("lib_dir", lib_dir)?;
        check(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_litert_dispatch_lib_dir(
                    self.as_ptr(),
                    lib_dir.as_ptr(),
                )
            },
            "litert_lm_embedding_engine_settings_set_litert_dispatch_lib_dir",
        )?;
        Ok(self)
    }

    /// Wraps
    /// `litert_lm_embedding_engine_settings_set_vision_litert_dispatch_lib_dir`.
    /// No effect if no vision backend is configured.
    pub fn set_vision_litert_dispatch_lib_dir(&mut self, lib_dir: &str) -> Result<&mut Self> {
        let lib_dir = cstr("lib_dir", lib_dir)?;
        check(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_vision_litert_dispatch_lib_dir(
                    self.as_ptr(),
                    lib_dir.as_ptr(),
                )
            },
            "litert_lm_embedding_engine_settings_set_vision_litert_dispatch_lib_dir",
        )?;
        Ok(self)
    }

    /// Wraps
    /// `litert_lm_embedding_engine_settings_set_audio_litert_dispatch_lib_dir`.
    /// No effect if no audio backend is configured.
    pub fn set_audio_litert_dispatch_lib_dir(&mut self, lib_dir: &str) -> Result<&mut Self> {
        let lib_dir = cstr("lib_dir", lib_dir)?;
        check(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_audio_litert_dispatch_lib_dir(
                    self.as_ptr(),
                    lib_dir.as_ptr(),
                )
            },
            "litert_lm_embedding_engine_settings_set_audio_litert_dispatch_lib_dir",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_embedding_engine_settings_set_max_input_length`.
    pub fn set_max_input_length(&mut self, max_input_length: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_max_input_length(
                    self.as_ptr(),
                    max_input_length,
                )
            },
            "litert_lm_embedding_engine_settings_set_max_input_length",
        );
        self
    }

    /// Wraps `litert_lm_embedding_engine_settings_set_min_input_length`.
    pub fn set_min_input_length(&mut self, min_input_length: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_min_input_length(
                    self.as_ptr(),
                    min_input_length,
                )
            },
            "litert_lm_embedding_engine_settings_set_min_input_length",
        );
        self
    }

    /// Wraps `litert_lm_embedding_engine_settings_set_vision_tokens_per_image`.
    pub fn set_vision_tokens_per_image(&mut self, tokens: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_vision_tokens_per_image(
                    self.as_ptr(),
                    tokens,
                )
            },
            "litert_lm_embedding_engine_settings_set_vision_tokens_per_image",
        );
        self
    }

    /// Wraps `litert_lm_embedding_engine_settings_set_activation_data_type`.
    pub fn set_activation_data_type(&mut self, data_type: ActivationDataType) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_embedding_engine_settings_set_activation_data_type(
                    self.as_ptr(),
                    data_type.into(),
                )
            },
            "litert_lm_embedding_engine_settings_set_activation_data_type",
        );
        self
    }
}

// =======================================================================
// EmbeddingOptions
// =======================================================================

owned_handle!(
    /// Per-request embedding options. Wraps `LiteRtLmEmbeddingOptions`.
    EmbeddingOptions,
    LiteRtLmEmbeddingOptions,
    litert_lm_embedding_options_delete
);

impl EmbeddingOptions {
    /// Wraps `litert_lm_embedding_options_create`.
    pub fn new() -> Result<Self> {
        let ptr = create("litert_lm_embedding_options_create", |out| unsafe {
            sys::litert_lm_embedding_options_create(out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_embedding_options_set_normalize`.
    pub fn set_normalize(&mut self, normalize: bool) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_embedding_options_set_normalize(self.as_ptr(), normalize) },
            "litert_lm_embedding_options_set_normalize",
        );
        self
    }

    /// Wraps `litert_lm_embedding_options_get_normalize`.
    pub fn normalize(&self) -> Result<bool> {
        out_value("litert_lm_embedding_options_get_normalize", |out| unsafe {
            sys::litert_lm_embedding_options_get_normalize(self.as_ptr(), out)
        })
    }

    /// Wraps `litert_lm_embedding_options_set_insert_special_tokens`.
    pub fn set_insert_special_tokens(&mut self, insert: bool) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_embedding_options_set_insert_special_tokens(self.as_ptr(), insert)
            },
            "litert_lm_embedding_options_set_insert_special_tokens",
        );
        self
    }

    /// Wraps `litert_lm_embedding_options_get_insert_special_tokens`.
    pub fn insert_special_tokens(&self) -> Result<bool> {
        out_value(
            "litert_lm_embedding_options_get_insert_special_tokens",
            |out| unsafe {
                sys::litert_lm_embedding_options_get_insert_special_tokens(self.as_ptr(), out)
            },
        )
    }

    /// Wraps `litert_lm_embedding_options_set_input_overflow_strategy`.
    pub fn set_input_overflow_strategy(&mut self, strategy: InputOverflowStrategy) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_embedding_options_set_input_overflow_strategy(
                    self.as_ptr(),
                    strategy.into(),
                )
            },
            "litert_lm_embedding_options_set_input_overflow_strategy",
        );
        self
    }

    /// Wraps `litert_lm_embedding_options_get_input_overflow_strategy`.
    pub fn input_overflow_strategy(&self) -> Result<InputOverflowStrategy> {
        const WHAT: &str = "litert_lm_embedding_options_get_input_overflow_strategy";
        let raw: sys::LiteRtLmInputOverflowStrategy = out_value(WHAT, |out| unsafe {
            sys::litert_lm_embedding_options_get_input_overflow_strategy(self.as_ptr(), out)
        })?;
        InputOverflowStrategy::from_raw(raw).ok_or(Error::UnexpectedValue {
            what: WHAT,
            value: raw as i64,
        })
    }

    /// Wraps `litert_lm_embedding_options_set_output_size`.
    pub fn set_output_size(&mut self, output_size: i32) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_embedding_options_set_output_size(self.as_ptr(), output_size) },
            "litert_lm_embedding_options_set_output_size",
        );
        self
    }

    /// Wraps `litert_lm_embedding_options_get_output_size`.
    pub fn output_size(&self) -> Result<i32> {
        out_value("litert_lm_embedding_options_get_output_size", |out| unsafe {
            sys::litert_lm_embedding_options_get_output_size(self.as_ptr(), out)
        })
    }

    /// Wraps `litert_lm_embedding_options_set_vision_tokens_per_image`.
    pub fn set_vision_tokens_per_image(&mut self, tokens: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_embedding_options_set_vision_tokens_per_image(self.as_ptr(), tokens)
            },
            "litert_lm_embedding_options_set_vision_tokens_per_image",
        );
        self
    }

    /// Wraps `litert_lm_embedding_options_get_vision_tokens_per_image`.
    pub fn vision_tokens_per_image(&self) -> Result<i32> {
        out_value(
            "litert_lm_embedding_options_get_vision_tokens_per_image",
            |out| unsafe {
                sys::litert_lm_embedding_options_get_vision_tokens_per_image(self.as_ptr(), out)
            },
        )
    }
}

// =======================================================================
// EmbeddingResponse / EmbeddingResponseRef / EmbeddingResponses
// =======================================================================

/// Shared accessors for owned and borrowed embedding responses.
macro_rules! embedding_response_accessors {
    () => {
        /// Wraps `litert_lm_embedding_response_get_size`: the number of
        /// values (the embedding dimension).
        pub fn size(&self) -> Result<usize> {
            out_value("litert_lm_embedding_response_get_size", |out| unsafe {
                sys::litert_lm_embedding_response_get_size(self.raw(), out)
            })
        }

        /// Wraps `litert_lm_embedding_response_get_values`: the embedding
        /// vector, borrowed from the response without copying.
        pub fn values(&self) -> Result<&[f32]> {
            let size = self.size()?;
            let mut ptr: *const f32 = std::ptr::null();
            check(
                unsafe { sys::litert_lm_embedding_response_get_values(self.raw(), &mut ptr) },
                "litert_lm_embedding_response_get_values",
            )?;
            if size == 0 {
                return Ok(&[]);
            }
            if ptr.is_null() {
                return Err(Error::Null("litert_lm_embedding_response_get_values"));
            }
            // SAFETY: `ptr` points at `size` floats owned by the response,
            // which outlives `&self`.
            Ok(unsafe { std::slice::from_raw_parts(ptr, size) })
        }
    };
}

owned_handle!(
    /// One embedding vector. Wraps `LiteRtLmEmbeddingResponse`.
    EmbeddingResponse,
    LiteRtLmEmbeddingResponse,
    litert_lm_embedding_response_delete
);

impl EmbeddingResponse {
    fn raw(&self) -> *const sys::LiteRtLmEmbeddingResponse {
        self.as_ptr()
    }

    embedding_response_accessors!();
}

/// An embedding vector **borrowed from an [`EmbeddingResponses`]** batch.
#[derive(Clone, Copy)]
pub struct EmbeddingResponseRef<'a> {
    ptr: *const sys::LiteRtLmEmbeddingResponse,
    _batch: PhantomData<&'a EmbeddingResponses>,
}

impl<'a> EmbeddingResponseRef<'a> {
    fn raw(&self) -> *const sys::LiteRtLmEmbeddingResponse {
        self.ptr
    }

    embedding_response_accessors!();
}

impl std::fmt::Debug for EmbeddingResponseRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingResponseRef").finish_non_exhaustive()
    }
}

owned_handle!(
    /// A batch of embedding vectors. Wraps `LiteRtLmEmbeddingResponses`.
    EmbeddingResponses,
    LiteRtLmEmbeddingResponses,
    litert_lm_embedding_responses_delete
);

impl EmbeddingResponses {
    /// Wraps `litert_lm_embedding_responses_get_size`.
    pub fn len(&self) -> Result<usize> {
        out_value("litert_lm_embedding_responses_get_size", |out| unsafe {
            sys::litert_lm_embedding_responses_get_size(self.as_ptr(), out)
        })
    }

    /// Returns `true` if the batch is empty.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Wraps `litert_lm_embedding_responses_get_at`. Borrows from this
    /// batch.
    pub fn get(&self, index: usize) -> Result<EmbeddingResponseRef<'_>> {
        let mut out: *const sys::LiteRtLmEmbeddingResponse = std::ptr::null();
        check(
            unsafe { sys::litert_lm_embedding_responses_get_at(self.as_ptr(), index, &mut out) },
            "litert_lm_embedding_responses_get_at",
        )?;
        if out.is_null() {
            return Err(Error::Null("litert_lm_embedding_responses_get_at"));
        }
        Ok(EmbeddingResponseRef {
            ptr: out,
            _batch: PhantomData,
        })
    }
}

// =======================================================================
// EmbeddingEngine
// =======================================================================

owned_handle!(
    /// An embedding engine. Wraps `LiteRtLmEmbeddingEngine`. Not `Sync`:
    /// calls on one engine must not overlap.
    EmbeddingEngine,
    LiteRtLmEmbeddingEngine,
    litert_lm_embedding_engine_delete
);

impl EmbeddingEngine {
    /// Wraps `litert_lm_embedding_engine_create`.
    pub fn new(settings: &EmbeddingEngineSettings) -> Result<Self> {
        let ptr = create("litert_lm_embedding_engine_create", |out| unsafe {
            sys::litert_lm_embedding_engine_create(settings.as_ptr(), out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_embedding_engine_compute_embedding`. `options` of
    /// `None` uses the defaults.
    pub fn compute_embedding(
        &self,
        inputs: &[InputData],
        options: Option<&EmbeddingOptions>,
    ) -> Result<EmbeddingResponse> {
        let ptrs = input_ptrs(inputs);
        let options_ptr = options.map_or(std::ptr::null(), |o| o.as_ptr() as *const _);
        let ptr = create("litert_lm_embedding_engine_compute_embedding", |out| unsafe {
            sys::litert_lm_embedding_engine_compute_embedding(
                self.as_ptr(),
                ptrs.as_ptr(),
                ptrs.len(),
                options_ptr,
                out,
            )
        })?;
        Ok(EmbeddingResponse::from_non_null(ptr))
    }

    /// Wraps `litert_lm_embedding_engine_compute_embedding_batch`: one
    /// embedding per element of `batch`, where each element is the list of
    /// inputs for that request.
    pub fn compute_embedding_batch(
        &self,
        batch: &[&[InputData]],
        options: Option<&EmbeddingOptions>,
    ) -> Result<EmbeddingResponses> {
        // Keep the per-request pointer arrays alive across the FFI call.
        let per_request: Vec<Vec<*const sys::LiteRtLmInputData>> =
            batch.iter().map(|inputs| input_ptrs(inputs)).collect();
        let request_ptrs: Vec<*const *const sys::LiteRtLmInputData> =
            per_request.iter().map(|v| v.as_ptr()).collect();
        let counts: Vec<usize> = per_request.iter().map(Vec::len).collect();
        let options_ptr = options.map_or(std::ptr::null(), |o| o.as_ptr() as *const _);

        let ptr = create(
            "litert_lm_embedding_engine_compute_embedding_batch",
            |out| unsafe {
                sys::litert_lm_embedding_engine_compute_embedding_batch(
                    self.as_ptr(),
                    request_ptrs.as_ptr(),
                    counts.as_ptr(),
                    request_ptrs.len(),
                    options_ptr,
                    out,
                )
            },
        )?;
        Ok(EmbeddingResponses::from_non_null(ptr))
    }
}

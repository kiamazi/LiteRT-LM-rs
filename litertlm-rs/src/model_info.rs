//! Model metadata (`model_info.h`): inspect a `.litertlm` file's
//! capabilities without creating an engine.
//!
//! ```no_run
//! use litertlm_rs::{LoadedFile, Modality};
//!
//! # fn main() -> litertlm_rs::Result<()> {
//! let file = LoadedFile::open("model.litertlm")?;
//! println!("thinking: {}", file.supports_thinking()?);
//! println!("vision:   {}", file.supports_input_modality(Modality::Vision)?);
//! # Ok(())
//! # }
//! ```
//!
//! Properties a model does not define (for example sampler defaults of an
//! embedding model) are reported by the C API as `NotFound`. Getters where
//! that is a normal outcome return `Result<Option<_>>` and map it to
//! `None`.

use litertlm_sys as sys;

use crate::error::{Error, Result};
use crate::types::*;
use crate::util::*;
use crate::version::check_api_version;

owned_handle!(
    /// A `.litertlm` file loaded for capability queries. Wraps
    /// `LiteRtLmLoadedFile`.
    LoadedFile,
    LiteRtLmLoadedFile,
    litert_lm_loaded_file_delete
);

/// Maps a `NotFound` error to `Ok(None)`.
fn not_found_as_none<T>(r: Result<T>) -> Result<Option<T>> {
    match r {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.is_not_found() => Ok(None),
        Err(e) => Err(e),
    }
}

impl LoadedFile {
    /// Wraps `litert_lm_loaded_file_create`.
    pub fn open(litertlm_path: &str) -> Result<Self> {
        check_api_version()?;
        let path = cstr("litertlm_path", litertlm_path)?;
        let ptr = create("litert_lm_loaded_file_create", |out| unsafe {
            sys::litert_lm_loaded_file_create(path.as_ptr(), out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    fn raw(&self) -> *const sys::LiteRtLmLoadedFile {
        self.as_ptr()
    }

    /// Wraps `litert_lm_loaded_file_has_speculative_decoding_support`.
    pub fn has_speculative_decoding_support(&self) -> Result<bool> {
        out_value(
            "litert_lm_loaded_file_has_speculative_decoding_support",
            |out| unsafe {
                sys::litert_lm_loaded_file_has_speculative_decoding_support(self.raw(), out)
            },
        )
    }

    /// Wraps `litert_lm_loaded_file_supports_thinking`.
    pub fn supports_thinking(&self) -> Result<bool> {
        out_value("litert_lm_loaded_file_supports_thinking", |out| unsafe {
            sys::litert_lm_loaded_file_supports_thinking(self.raw(), out)
        })
    }

    /// Wraps `litert_lm_loaded_file_supports_function_calling`.
    pub fn supports_function_calling(&self) -> Result<bool> {
        out_value(
            "litert_lm_loaded_file_supports_function_calling",
            |out| unsafe {
                sys::litert_lm_loaded_file_supports_function_calling(self.raw(), out)
            },
        )
    }

    /// Wraps `litert_lm_loaded_file_sampler_type`: the model's default
    /// sampler. `None` if the model is not an LLM.
    pub fn sampler_type(&self) -> Result<Option<SamplerType>> {
        const WHAT: &str = "litert_lm_loaded_file_sampler_type";
        let raw = not_found_as_none(out_value::<sys::LiteRtLmSamplerType>(WHAT, |out| unsafe {
            sys::litert_lm_loaded_file_sampler_type(self.raw(), out)
        }))?;
        match raw {
            None => Ok(None),
            Some(raw) => SamplerType::from_raw(raw)
                .map(Some)
                .ok_or(Error::UnexpectedValue {
                    what: WHAT,
                    value: raw as i64,
                }),
        }
    }

    /// Wraps `litert_lm_loaded_file_sampler_temperature`. `None` if the
    /// model is not an LLM.
    pub fn sampler_temperature(&self) -> Result<Option<f32>> {
        not_found_as_none(out_value(
            "litert_lm_loaded_file_sampler_temperature",
            |out| unsafe { sys::litert_lm_loaded_file_sampler_temperature(self.raw(), out) },
        ))
    }

    /// Wraps `litert_lm_loaded_file_sampler_top_k`. `None` if the model is
    /// not an LLM.
    pub fn sampler_top_k(&self) -> Result<Option<i32>> {
        not_found_as_none(out_value("litert_lm_loaded_file_sampler_top_k", |out| unsafe {
            sys::litert_lm_loaded_file_sampler_top_k(self.raw(), out)
        }))
    }

    /// Wraps `litert_lm_loaded_file_sampler_top_p`. `None` if the model is
    /// not an LLM.
    pub fn sampler_top_p(&self) -> Result<Option<f32>> {
        not_found_as_none(out_value("litert_lm_loaded_file_sampler_top_p", |out| unsafe {
            sys::litert_lm_loaded_file_sampler_top_p(self.raw(), out)
        }))
    }

    /// Wraps `litert_lm_loaded_file_supports_input_modality`.
    pub fn supports_input_modality(&self, modality: Modality) -> Result<bool> {
        out_value(
            "litert_lm_loaded_file_supports_input_modality",
            |out| unsafe {
                sys::litert_lm_loaded_file_supports_input_modality(
                    self.raw(),
                    modality.into(),
                    out,
                )
            },
        )
    }

    /// Wraps `litert_lm_loaded_file_max_vision_token_budget`. `None` if the
    /// model does not support vision or does not define the budget.
    pub fn max_vision_token_budget(&self) -> Result<Option<i32>> {
        not_found_as_none(out_value(
            "litert_lm_loaded_file_max_vision_token_budget",
            |out| unsafe { sys::litert_lm_loaded_file_max_vision_token_budget(self.raw(), out) },
        ))
    }

    /// Wraps `litert_lm_loaded_file_max_context_tokens`. `None` if the
    /// model does not define its context size.
    pub fn max_context_tokens(&self) -> Result<Option<u32>> {
        not_found_as_none(out_value(
            "litert_lm_loaded_file_max_context_tokens",
            |out| unsafe { sys::litert_lm_loaded_file_max_context_tokens(self.raw(), out) },
        ))
    }

    /// Wraps `litert_lm_loaded_file_is_dynamic_context`.
    pub fn is_dynamic_context(&self) -> Result<bool> {
        out_value("litert_lm_loaded_file_is_dynamic_context", |out| unsafe {
            sys::litert_lm_loaded_file_is_dynamic_context(self.raw(), out)
        })
    }

    /// Wraps `litert_lm_loaded_file_vision_signature_selection`: the
    /// discrete vision token budgets supported by each vision signature.
    /// `None` if the model does not support vision or does not define them.
    pub fn vision_signature_selection(&self) -> Result<Option<Vec<i32>>> {
        not_found_as_none(self.fill_i32_list(
            "litert_lm_loaded_file_vision_signature_selection",
            |buf, cap, count| unsafe {
                sys::litert_lm_loaded_file_vision_signature_selection(self.raw(), buf, cap, count)
            },
        ))
    }

    /// Wraps `litert_lm_loaded_file_modality_supported_backends`: the
    /// supported backends for `modality`, ordered by priority (first entry
    /// is the default). Empty if the modality is not supported.
    pub fn modality_supported_backends(&self, modality: Modality) -> Result<Vec<BackendType>> {
        const WHAT: &str = "litert_lm_loaded_file_modality_supported_backends";
        // Two-call pattern: query the count, then fill a buffer of that size.
        let count: usize = out_value(WHAT, |out| unsafe {
            sys::litert_lm_loaded_file_modality_supported_backends(
                self.raw(),
                modality.into(),
                std::ptr::null_mut(),
                0,
                out,
            )
        })?;
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut buf: Vec<sys::LiteRtLmBackendType> = vec![0; count];
        let mut written = 0usize;
        check(
            unsafe {
                sys::litert_lm_loaded_file_modality_supported_backends(
                    self.raw(),
                    modality.into(),
                    buf.as_mut_ptr(),
                    buf.len(),
                    &mut written,
                )
            },
            WHAT,
        )?;
        buf.truncate(written.min(buf.len()));
        buf.into_iter()
            .map(|raw| {
                BackendType::from_raw(raw).ok_or(Error::UnexpectedValue {
                    what: WHAT,
                    value: raw as i64,
                })
            })
            .collect()
    }

    /// Wraps `litert_lm_loaded_file_modality_npu_brand`.
    pub fn modality_npu_brand(&self, modality: Modality) -> Result<NpuBrand> {
        let raw: sys::LiteRtLmNpuBrand =
            out_value("litert_lm_loaded_file_modality_npu_brand", |out| unsafe {
                sys::litert_lm_loaded_file_modality_npu_brand(self.raw(), modality.into(), out)
            })?;
        Ok(NpuBrand::from_raw(raw))
    }

    /// Wraps `litert_lm_loaded_file_modality_soc_name`. `None` if no SoC
    /// name is specified.
    pub fn modality_soc_name(&self, modality: Modality) -> Result<Option<String>> {
        out_string("litert_lm_loaded_file_modality_soc_name", |out| unsafe {
            sys::litert_lm_loaded_file_modality_soc_name(self.raw(), modality.into(), out)
        })
    }

    /// Wraps `litert_lm_loaded_file_min_runtime_version`. `None` if the
    /// file defines no version requirement.
    pub fn min_runtime_version(&self) -> Result<Option<String>> {
        out_string("litert_lm_loaded_file_min_runtime_version", |out| unsafe {
            sys::litert_lm_loaded_file_min_runtime_version(self.raw(), out)
        })
    }

    /// Wraps `litert_lm_loaded_file_model_type`.
    pub fn model_type(&self) -> Result<ModelType> {
        let raw: sys::LiteRtLmModelType =
            out_value("litert_lm_loaded_file_model_type", |out| unsafe {
                sys::litert_lm_loaded_file_model_type(self.raw(), out)
            })?;
        Ok(ModelType::from_raw(raw))
    }

    /// Wraps `litert_lm_loaded_file_embedding_dimension`. `None` if the
    /// model is not an embedding model or does not define the dimension.
    pub fn embedding_dimension(&self) -> Result<Option<i32>> {
        not_found_as_none(out_value(
            "litert_lm_loaded_file_embedding_dimension",
            |out| unsafe { sys::litert_lm_loaded_file_embedding_dimension(self.raw(), out) },
        ))
    }

    /// Wraps `litert_lm_loaded_file_embedding_signature_selection`. `None`
    /// if the model is not an embedding model or does not define signature
    /// lengths.
    pub fn embedding_signature_selection(&self) -> Result<Option<Vec<i32>>> {
        not_found_as_none(self.fill_i32_list(
            "litert_lm_loaded_file_embedding_signature_selection",
            |buf, cap, count| unsafe {
                sys::litert_lm_loaded_file_embedding_signature_selection(
                    self.raw(),
                    buf,
                    cap,
                    count,
                )
            },
        ))
    }

    /// Implements the C API's "query the count, then fill a buffer" pattern
    /// for `int32_t` lists.
    fn fill_i32_list(
        &self,
        what: &'static str,
        call: impl Fn(*mut i32, usize, *mut usize) -> sys::LiteRtLmStatusCode,
    ) -> Result<Vec<i32>> {
        let count: usize = out_value(what, |out| call(std::ptr::null_mut(), 0, out))?;
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut buf = vec![0i32; count];
        let mut written = 0usize;
        check(call(buf.as_mut_ptr(), buf.len(), &mut written), what)?;
        buf.truncate(written.min(buf.len()));
        Ok(buf)
    }
}

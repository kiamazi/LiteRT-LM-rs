//! Result and info objects returned by the library: responses, benchmark
//! info, tokenizer results and token unions.

use std::ffi::c_int;
use std::marker::PhantomData;
use std::ptr::NonNull;

use litertlm_sys as sys;

use crate::error::{Error, Result};
use crate::types::TokenUnionType;
use crate::util::*;

// =======================================================================
// Responses
// =======================================================================

owned_handle!(
    /// Candidate responses from a session. Wraps `LiteRtLmResponses`.
    Responses,
    LiteRtLmResponses,
    litert_lm_responses_delete
);

impl Responses {
    /// Wraps `litert_lm_responses_get_num_candidates`.
    pub fn num_candidates(&self) -> Result<usize> {
        out_value("litert_lm_responses_get_num_candidates", |out| unsafe {
            sys::litert_lm_responses_get_num_candidates(self.as_ptr(), out)
        })
    }

    /// Wraps `litert_lm_responses_get_response_text_at`. Errors with
    /// `OutOfRange` for a bad `index` and `NotFound` if there is no text at
    /// `index`.
    pub fn response_text_at(&self, index: usize) -> Result<String> {
        out_string_required("litert_lm_responses_get_response_text_at", |out| unsafe {
            sys::litert_lm_responses_get_response_text_at(self.as_ptr(), index, out)
        })
    }

    /// The text of every candidate, in order.
    pub fn texts(&self) -> Result<Vec<String>> {
        (0..self.num_candidates()?)
            .map(|i| self.response_text_at(i))
            .collect()
    }

    /// Wraps `litert_lm_responses_has_score_at` +
    /// `litert_lm_responses_get_score_at`. `None` if the candidate has no
    /// score.
    pub fn score_at(&self, index: usize) -> Result<Option<f32>> {
        let has: bool = out_value("litert_lm_responses_has_score_at", |out| unsafe {
            sys::litert_lm_responses_has_score_at(self.as_ptr(), index, out)
        })?;
        if !has {
            return Ok(None);
        }
        out_value("litert_lm_responses_get_score_at", |out| unsafe {
            sys::litert_lm_responses_get_score_at(self.as_ptr(), index, out)
        })
        .map(Some)
    }

    /// Wraps `litert_lm_responses_has_token_length_at` +
    /// `litert_lm_responses_get_token_length_at`.
    pub fn token_length_at(&self, index: usize) -> Result<Option<usize>> {
        let has: bool = out_value("litert_lm_responses_has_token_length_at", |out| unsafe {
            sys::litert_lm_responses_has_token_length_at(self.as_ptr(), index, out)
        })?;
        if !has {
            return Ok(None);
        }
        out_value("litert_lm_responses_get_token_length_at", |out| unsafe {
            sys::litert_lm_responses_get_token_length_at(self.as_ptr(), index, out)
        })
        .map(Some)
    }

    /// Wraps `litert_lm_responses_has_token_scores_at` +
    /// `litert_lm_responses_get_num_token_scores_at` +
    /// `litert_lm_responses_get_token_scores_at`, copied into an owned
    /// `Vec` (the raw pointer is only valid for this object's lifetime).
    pub fn token_scores_at(&self, index: usize) -> Result<Option<Vec<f32>>> {
        let has: bool = out_value("litert_lm_responses_has_token_scores_at", |out| unsafe {
            sys::litert_lm_responses_has_token_scores_at(self.as_ptr(), index, out)
        })?;
        if !has {
            return Ok(None);
        }
        let num: usize = out_value("litert_lm_responses_get_num_token_scores_at", |out| unsafe {
            sys::litert_lm_responses_get_num_token_scores_at(self.as_ptr(), index, out)
        })?;
        let mut ptr: *const f32 = std::ptr::null();
        check(
            unsafe {
                sys::litert_lm_responses_get_token_scores_at(self.as_ptr(), index, &mut ptr)
            },
            "litert_lm_responses_get_token_scores_at",
        )?;
        if ptr.is_null() || num == 0 {
            return Ok(Some(Vec::new()));
        }
        // SAFETY: `ptr` points at `num` floats owned by `self`.
        Ok(Some(unsafe { std::slice::from_raw_parts(ptr, num) }.to_vec()))
    }
}

// =======================================================================
// BenchmarkInfo
// =======================================================================

owned_handle!(
    /// Benchmark data. Requires
    /// [`EngineSettings::enable_benchmark`](crate::EngineSettings::enable_benchmark)
    /// before the engine is created. Wraps `LiteRtLmBenchmarkInfo`.
    BenchmarkInfo,
    LiteRtLmBenchmarkInfo,
    litert_lm_benchmark_info_delete
);

impl BenchmarkInfo {
    /// Wraps `litert_lm_benchmark_info_get_time_to_first_token`. Seconds;
    /// excludes initialization time.
    pub fn time_to_first_token(&self) -> Result<f64> {
        out_value(
            "litert_lm_benchmark_info_get_time_to_first_token",
            |out| unsafe {
                sys::litert_lm_benchmark_info_get_time_to_first_token(self.as_ptr(), out)
            },
        )
    }

    /// Wraps `litert_lm_benchmark_info_get_total_init_time_in_second`.
    pub fn total_init_time_in_second(&self) -> Result<f64> {
        out_value(
            "litert_lm_benchmark_info_get_total_init_time_in_second",
            |out| unsafe {
                sys::litert_lm_benchmark_info_get_total_init_time_in_second(self.as_ptr(), out)
            },
        )
    }

    /// Wraps `litert_lm_benchmark_info_get_num_prefill_turns`.
    pub fn num_prefill_turns(&self) -> Result<usize> {
        out_value(
            "litert_lm_benchmark_info_get_num_prefill_turns",
            |out| unsafe {
                sys::litert_lm_benchmark_info_get_num_prefill_turns(self.as_ptr(), out)
            },
        )
    }

    /// Wraps `litert_lm_benchmark_info_get_num_decode_turns`.
    pub fn num_decode_turns(&self) -> Result<usize> {
        out_value(
            "litert_lm_benchmark_info_get_num_decode_turns",
            |out| unsafe { sys::litert_lm_benchmark_info_get_num_decode_turns(self.as_ptr(), out) },
        )
    }

    /// Wraps `litert_lm_benchmark_info_get_prefill_token_count_at`.
    pub fn prefill_token_count_at(&self, index: usize) -> Result<usize> {
        out_value(
            "litert_lm_benchmark_info_get_prefill_token_count_at",
            |out| unsafe {
                sys::litert_lm_benchmark_info_get_prefill_token_count_at(self.as_ptr(), index, out)
            },
        )
    }

    /// Wraps `litert_lm_benchmark_info_get_decode_token_count_at`.
    pub fn decode_token_count_at(&self, index: usize) -> Result<usize> {
        out_value(
            "litert_lm_benchmark_info_get_decode_token_count_at",
            |out| unsafe {
                sys::litert_lm_benchmark_info_get_decode_token_count_at(self.as_ptr(), index, out)
            },
        )
    }

    /// Wraps `litert_lm_benchmark_info_get_prefill_tokens_per_sec_at`.
    pub fn prefill_tokens_per_sec_at(&self, index: usize) -> Result<f64> {
        out_value(
            "litert_lm_benchmark_info_get_prefill_tokens_per_sec_at",
            |out| unsafe {
                sys::litert_lm_benchmark_info_get_prefill_tokens_per_sec_at(
                    self.as_ptr(),
                    index,
                    out,
                )
            },
        )
    }

    /// Wraps `litert_lm_benchmark_info_get_decode_tokens_per_sec_at`.
    pub fn decode_tokens_per_sec_at(&self, index: usize) -> Result<f64> {
        out_value(
            "litert_lm_benchmark_info_get_decode_tokens_per_sec_at",
            |out| unsafe {
                sys::litert_lm_benchmark_info_get_decode_tokens_per_sec_at(
                    self.as_ptr(),
                    index,
                    out,
                )
            },
        )
    }

    /// Wraps `litert_lm_benchmark_info_get_max_num_tokens` (new in C API
    /// 1.0.0).
    pub fn max_num_tokens(&self) -> Result<i32> {
        out_value(
            "litert_lm_benchmark_info_get_max_num_tokens",
            |out| unsafe { sys::litert_lm_benchmark_info_get_max_num_tokens(self.as_ptr(), out) },
        )
    }
}

// =======================================================================
// TokenizeResult / DetokenizeResult
// =======================================================================

owned_handle!(
    /// Wraps `LiteRtLmTokenizeResult`.
    TokenizeResult,
    LiteRtLmTokenizeResult,
    litert_lm_tokenize_result_delete
);

impl TokenizeResult {
    /// Wraps `litert_lm_tokenize_result_get_tokens` +
    /// `litert_lm_tokenize_result_get_num_tokens`, copied into an owned
    /// `Vec`.
    pub fn tokens(&self) -> Result<Vec<i32>> {
        let num: usize = out_value("litert_lm_tokenize_result_get_num_tokens", |out| unsafe {
            sys::litert_lm_tokenize_result_get_num_tokens(self.as_ptr(), out)
        })?;
        let mut ptr: *const c_int = std::ptr::null();
        check(
            unsafe { sys::litert_lm_tokenize_result_get_tokens(self.as_ptr(), &mut ptr) },
            "litert_lm_tokenize_result_get_tokens",
        )?;
        if ptr.is_null() || num == 0 {
            Ok(Vec::new())
        } else {
            // SAFETY: `ptr` points at `num` ints owned by `self`.
            Ok(unsafe { std::slice::from_raw_parts(ptr, num) }.to_vec())
        }
    }
}

owned_handle!(
    /// Wraps `LiteRtLmDetokenizeResult`.
    DetokenizeResult,
    LiteRtLmDetokenizeResult,
    litert_lm_detokenize_result_delete
);

impl DetokenizeResult {
    /// Wraps `litert_lm_detokenize_result_get_string`.
    pub fn text(&self) -> Result<String> {
        out_string_required("litert_lm_detokenize_result_get_string", |out| unsafe {
            sys::litert_lm_detokenize_result_get_string(self.as_ptr(), out)
        })
    }
}

// =======================================================================
// TokenUnion (owned) / TokenUnionRef (borrowed) / TokenUnions
// =======================================================================

/// Implements the shared read accessors for owned and borrowed token unions.
macro_rules! token_union_accessors {
    () => {
        /// Wraps `litert_lm_token_union_get_type`.
        pub fn union_type(&self) -> Result<TokenUnionType> {
            let raw: sys::LiteRtLmTokenUnionType =
                out_value("litert_lm_token_union_get_type", |out| unsafe {
                    sys::litert_lm_token_union_get_type(self.raw(), out)
                })?;
            TokenUnionType::from_raw(raw).ok_or(Error::UnexpectedValue {
                what: "litert_lm_token_union_get_type",
                value: raw as i64,
            })
        }

        /// Wraps `litert_lm_token_union_get_string`. `None` if this token
        /// union holds token ids instead of a string.
        pub fn as_string(&self) -> Result<Option<String>> {
            if self.union_type()? != TokenUnionType::String {
                return Ok(None);
            }
            out_string("litert_lm_token_union_get_string", |out| unsafe {
                sys::litert_lm_token_union_get_string(self.raw(), out)
            })
        }

        /// Wraps `litert_lm_token_union_get_ids`, copied into an owned
        /// `Vec`. `None` if this token union holds a string instead of ids.
        pub fn as_ids(&self) -> Result<Option<Vec<i32>>> {
            if self.union_type()? != TokenUnionType::Ids {
                return Ok(None);
            }
            let mut tokens: *const c_int = std::ptr::null();
            let mut num: usize = 0;
            check(
                unsafe { sys::litert_lm_token_union_get_ids(self.raw(), &mut tokens, &mut num) },
                "litert_lm_token_union_get_ids",
            )?;
            if tokens.is_null() || num == 0 {
                Ok(Some(Vec::new()))
            } else {
                // SAFETY: `tokens` points at `num` ints owned by the union.
                Ok(Some(unsafe { std::slice::from_raw_parts(tokens, num) }.to_vec()))
            }
        }
    };
}

/// A token union **owned by the caller**, such as the one returned by
/// [`Engine::get_start_token`](crate::Engine::get_start_token). Wraps
/// `LiteRtLmTokenUnion`; released with `litert_lm_token_union_delete`.
pub struct TokenUnion {
    ptr: NonNull<sys::LiteRtLmTokenUnion>,
}

// SAFETY: see `owned_handle!`.
unsafe impl Send for TokenUnion {}

impl TokenUnion {
    pub(crate) fn from_non_null(ptr: NonNull<sys::LiteRtLmTokenUnion>) -> Self {
        Self { ptr }
    }

    fn raw(&self) -> *const sys::LiteRtLmTokenUnion {
        self.ptr.as_ptr()
    }

    token_union_accessors!();
}

impl Drop for TokenUnion {
    fn drop(&mut self) {
        unsafe { sys::litert_lm_token_union_delete(self.ptr.as_ptr()) };
    }
}

impl std::fmt::Debug for TokenUnion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenUnion").finish_non_exhaustive()
    }
}

/// A token union **borrowed from a [`TokenUnions`] collection**. It is owned
/// by the collection and must not be (and cannot be) deleted separately; the
/// borrow checker ties it to the collection's lifetime.
#[derive(Clone, Copy)]
pub struct TokenUnionRef<'a> {
    ptr: *const sys::LiteRtLmTokenUnion,
    _collection: PhantomData<&'a TokenUnions>,
}

impl<'a> TokenUnionRef<'a> {
    fn raw(&self) -> *const sys::LiteRtLmTokenUnion {
        self.ptr
    }

    token_union_accessors!();
}

impl std::fmt::Debug for TokenUnionRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenUnionRef").finish_non_exhaustive()
    }
}

owned_handle!(
    /// A collection of token unions, e.g. an engine's stop tokens. Wraps
    /// `LiteRtLmTokenUnions`.
    TokenUnions,
    LiteRtLmTokenUnions,
    litert_lm_token_unions_delete
);

impl TokenUnions {
    /// Wraps `litert_lm_token_unions_get_num_tokens`.
    pub fn len(&self) -> Result<usize> {
        out_value("litert_lm_token_unions_get_num_tokens", |out| unsafe {
            sys::litert_lm_token_unions_get_num_tokens(self.as_ptr(), out)
        })
    }

    /// Returns `true` if this collection has no token unions.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    /// Wraps `litert_lm_token_unions_get_token_at`. The returned value
    /// borrows from this collection.
    pub fn get(&self, index: usize) -> Result<TokenUnionRef<'_>> {
        let mut out: *const sys::LiteRtLmTokenUnion = std::ptr::null();
        check(
            unsafe { sys::litert_lm_token_unions_get_token_at(self.as_ptr(), index, &mut out) },
            "litert_lm_token_unions_get_token_at",
        )?;
        if out.is_null() {
            return Err(Error::Null("litert_lm_token_unions_get_token_at"));
        }
        Ok(TokenUnionRef {
            ptr: out,
            _collection: PhantomData,
        })
    }

    /// Iterates over the token unions in this collection.
    pub fn iter(&self) -> Result<impl Iterator<Item = Result<TokenUnionRef<'_>>>> {
        let len = self.len()?;
        Ok((0..len).map(move |i| self.get(i)))
    }
}

//! Safe Rust bindings to the LiteRT-LM **C API 1.0.0**, covering every
//! public header of Google's official prebuilt package:
//!
//! | Header               | Rust API                                                              |
//! |----------------------|-----------------------------------------------------------------------|
//! | `engine.h`           | [`EngineSettings`], [`Engine`], [`Session`], sampler / decoding configs, tokenizer, streaming, benchmarking |
//! | `conversation.h`     | [`Conversation`], [`ConversationConfig`], [`ConversationOptionalArgs`] |
//! | `embedding_engine.h` | [`EmbeddingEngine`], [`EmbeddingEngineSettings`], [`EmbeddingOptions`] |
//! | `model_info.h`       | [`LoadedFile`]                                                        |
//! | `experimental.h`     | [`experimental`] (no stability guarantees)                            |
//! | `error_reporter.h`   | [`Error`], [`StatusCode`], [`last_error_message`]                     |
//!
//! # What changed in 1.0.0
//!
//! The C API is now fully status-code based, so nearly every operation
//! returns [`Result`]. Highlights:
//!
//! * [`Error`] carries the C status code **and** the library's thread-local
//!   error message, captured at the moment of failure.
//! * Sessions and conversations keep their [`Engine`] alive, so the C API's
//!   "destroy children before the engine" rule cannot be violated.
//! * After `cancel_process`, a session/conversation can only be dropped; this
//!   is enforced ([`Error::UseAfterCancel`]).
//! * The loaded library's C API version is verified once before the first
//!   engine is created ([`check_api_version`]).
//! * New: [`SpeculativeDecodingConfig`], [`Session::clone_session`],
//!   single-threaded execution, [`EmbeddingEngine`], [`LoadedFile`],
//!   [`experimental`].
//!
//! Setters that can only fail on a NULL pointer (impossible here) keep the
//! chainable `&mut Self` style; setters that can really fail return
//! `Result<&mut Self>`.
//!
//! See the README for a function-by-function list.

#[macro_use]
mod util;

mod config;
mod conversation;
mod embedding;
mod engine;
mod error;
pub mod experimental;
mod model_info;
mod results;
pub mod stream;
mod types;
mod version;

pub use litertlm_sys;

pub use config::{
    ConversationConfig, ConversationOptionalArgs, InputData, NoRepeatNgramConfig,
    RepetitionPenaltyConfig, SamplerParams, SessionConfig, SpeculativeDecodingConfig,
    SuppressTokensConfig, ThinkingConfig,
};
pub use conversation::Conversation;
pub use embedding::{
    EmbeddingEngine, EmbeddingEngineSettings, EmbeddingOptions, EmbeddingResponse,
    EmbeddingResponseRef, EmbeddingResponses,
};
pub use engine::{Engine, EngineSettings, Session};
pub use error::{clear_last_error, last_error_message, Error, Result, StatusCode};
pub use model_info::LoadedFile;
pub use results::{
    BenchmarkInfo, DetokenizeResult, Responses, TokenUnion, TokenUnionRef, TokenUnions,
    TokenizeResult,
};
pub use stream::StreamChunk;
pub use types::{
    ActivationDataType, BackendType, ConstraintProviderType, ConstraintType, InputDataType,
    InputOverflowStrategy, LogSeverity, Modality, ModelType, NpuBrand, SamplerType,
    SpeculativeDecodingMode, TokenUnionType,
};
pub use version::{api_version, check_api_version, ApiVersion};

use litertlm_sys as sys;

/// Sets the minimum log severity for the underlying C++ library. Call this
/// once, before creating an [`Engine`], to quiet (or increase) its stderr
/// logging. It writes straight to stderr, independent of anything in Rust,
/// so this is the only way to control it.
///
/// Wraps `litert_lm_set_min_log_level`.
pub fn set_min_log_level(level: LogSeverity) -> Result<()> {
    util::check(
        unsafe { sys::litert_lm_set_min_log_level(level.into()) },
        "litert_lm_set_min_log_level",
    )
}

/// Extracts and concatenates just the plain text from a message JSON object
/// of the shape `{"role": "...", "content": [{"type": "text", "text":
/// "..."}, ...]}` -- i.e. what [`Conversation::send_message`] or each stream
/// chunk's [`StreamChunk::text`] returns. Non-text content blocks are
/// skipped. Returns an empty string if the JSON doesn't match the expected
/// shape, since callers usually just want "whatever text is in here, if
/// any".
pub fn extract_text(message_json: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(message_json) else {
        return String::new();
    };
    let Some(content) = value.get("content").and_then(|c| c.as_array()) else {
        return String::new();
    };
    content
        .iter()
        .filter(|block| block.get("type").and_then(|t| t.as_str()) == Some("text"))
        .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("")
}

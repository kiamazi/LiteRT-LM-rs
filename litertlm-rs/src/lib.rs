//! Safe wrapper around `litert-lm-sys`, covering the full LiteRT LM C API
//! (`engine.h`): engine/session/conversation lifecycle, sampler and
//! decoding-control configs, streaming, tokenization, and benchmarking.
//!
//! Every exported `litert_lm_*` function in `engine.h` has a corresponding
//! safe wrapper here. See the README for a full function-by-function list.

use std::ffi::{c_void, CStr, CString, NulError};
use std::fmt;
use std::os::raw::{c_char, c_int};
pub use litertlm_sys;

// =======================================================================
// Errors
// =======================================================================

/// Errors returned by this crate.
#[derive(Debug)]
pub enum Error {
    /// A Rust string passed to the C API contained an interior NUL byte.
    NulArgument(&'static str),
    /// A `_create` function returned NULL.
    CreateFailed(&'static str),
    /// A function that returns a C "0 = success" status code returned non-zero.
    CallFailed(&'static str),
    /// The stream ended with an error chunk.
    Stream(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NulArgument(name) => write!(f, "argument `{name}` contained a NUL byte"),
            Error::CreateFailed(what) => write!(f, "{what} returned NULL"),
            Error::CallFailed(what) => write!(f, "{what} returned a failure status"),
            Error::Stream(msg) => write!(f, "stream error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Error::NulArgument("<string argument>")
    }
}

fn cstr(s: &str) -> Result<CString, Error> {
    CString::new(s).map_err(Into::into)
}

/// Reads an owned Rust `String` out of a C string pointer owned by some
/// other object (i.e. we must NOT free it ourselves). Returns `None` for
/// NULL.
unsafe fn owned_str_from(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

fn check_created<T>(ptr: *mut T, what: &'static str) -> Result<*mut T, Error> {
    if ptr.is_null() {
        Err(Error::CreateFailed(what))
    } else {
        Ok(ptr)
    }
}

fn check_status(ret: c_int, what: &'static str) -> Result<(), Error> {
    if ret != 0 {
        Err(Error::CallFailed(what))
    } else {
        Ok(())
    }
}

/// Generates a `pub struct $name { pub(crate) ptr: *mut sys::$sys_ty }` with
/// `Send`, an `as_ptr()` accessor, and a `Drop` impl calling `$delete_fn`.
/// Used for the many simple owned-handle types in this crate (configs,
/// results, etc.) to avoid repeating this boilerplate for each one.
macro_rules! opaque_owned {
    ($name:ident, $sys_ty:ident, $delete_fn:path) => {
        pub struct $name {
            pub(crate) ptr: *mut litertlm_sys::$sys_ty,
        }
        unsafe impl Send for $name {}
        #[allow(dead_code)]
        impl $name {
            pub(crate) fn as_ptr(&self) -> *const litertlm_sys::$sys_ty {
                self.ptr
            }
        }
        impl Drop for $name {
            fn drop(&mut self) {
                unsafe { $delete_fn(self.ptr) };
            }
        }
    };
}

// =======================================================================
// Logging
// =======================================================================

/// Minimum severity for the underlying C++ library's own logging (writes
/// straight to stderr, independent of anything in Rust -- this is the only
/// way to control it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSeverity {
    Verbose,
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
    /// Suppresses everything, including errors.
    Silent,
}

impl From<LogSeverity> for litertlm_sys::LiteRtLmLogSeverity {
    fn from(level: LogSeverity) -> Self {
        match level {
            LogSeverity::Verbose => litertlm_sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityVerbose,
            LogSeverity::Debug => litertlm_sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityDebug,
            LogSeverity::Info => litertlm_sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityInfo,
            LogSeverity::Warning => litertlm_sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityWarning,
            LogSeverity::Error => litertlm_sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityError,
            LogSeverity::Fatal => litertlm_sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityFatal,
            LogSeverity::Silent => litertlm_sys::LiteRtLmLogSeverity_kLiteRtLmLogSeveritySilent,
        }
    }
}

/// Sets the minimum log severity for the underlying C++ library. Call this
/// once, before creating an `Engine`, to quiet (or increase) its stderr
/// logging.
///
/// Wraps `litert_lm_set_min_log_level`.
pub fn set_min_log_level(level: LogSeverity) {
    unsafe { litertlm_sys::litert_lm_set_min_log_level(level.into()) };
}

// =======================================================================
// Message JSON helper
// =======================================================================

/// Extracts and concatenates just the plain text from a message JSON object
/// of the shape `{"role": "...", "content": [{"type": "text", "text":
/// "..."}, ...]}` -- i.e. what `Conversation::send_message` or each stream
/// chunk's `text()` returns. Non-text content blocks are skipped. Returns
/// an empty string if the JSON doesn't match the expected shape, since
/// callers usually just want "whatever text is in here, if any".
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

// =======================================================================
// Small enums shared across configs
// =======================================================================

/// Wraps `LiteRtLmSamplerType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerType {
    /// Probabilistically pick among the top-k tokens.
    TopK,
    /// Probabilistically pick among tokens whose cumulative probability
    /// reaches p, after first performing top-k sampling.
    TopP,
    /// Always pick the token with the maximum logit (argmax).
    Greedy,
}

impl From<SamplerType> for litertlm_sys::LiteRtLmSamplerType {
    fn from(t: SamplerType) -> Self {
        match t {
            SamplerType::TopK => litertlm_sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeTopK,
            SamplerType::TopP => litertlm_sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeTopP,
            SamplerType::Greedy => litertlm_sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeGreedy,
        }
    }
}

/// Wraps `LiteRtLmConstraintType`, for constrained decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    None,
    Regex,
    JsonSchema,
}

impl From<ConstraintType> for litertlm_sys::LiteRtLmConstraintType {
    fn from(t: ConstraintType) -> Self {
        match t {
            ConstraintType::None => litertlm_sys::LiteRtLmConstraintType_kLiteRtLmConstraintTypeNone,
            ConstraintType::Regex => litertlm_sys::LiteRtLmConstraintType_kLiteRtLmConstraintTypeRegex,
            ConstraintType::JsonSchema => {
                litertlm_sys::LiteRtLmConstraintType_kLiteRtLmConstraintTypeJsonSchema
            }
        }
    }
}

/// Wraps `LiteRtLmConstraintProviderType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintProviderType {
    LlGuidance,
}

impl From<ConstraintProviderType> for litertlm_sys::LiteRtLmConstraintProviderType {
    fn from(t: ConstraintProviderType) -> Self {
        match t {
            ConstraintProviderType::LlGuidance => {
                litertlm_sys::LiteRtLmConstraintProviderType_kLiteRtLmConstraintProviderTypeLlGuidance
            }
        }
    }
}

/// Wraps `LiteRtLmInputDataType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDataType {
    Text,
    Image,
    ImageEnd,
    Audio,
    AudioEnd,
}

impl From<InputDataType> for litertlm_sys::LiteRtLmInputDataType {
    fn from(t: InputDataType) -> Self {
        match t {
            InputDataType::Text => litertlm_sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeText,
            InputDataType::Image => litertlm_sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeImage,
            InputDataType::ImageEnd => {
                litertlm_sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeImageEnd
            }
            InputDataType::Audio => litertlm_sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeAudio,
            InputDataType::AudioEnd => {
                litertlm_sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeAudioEnd
            }
        }
    }
}

/// Wraps `LiteRtLmActivationDataType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationDataType {
    Float32,
    Float16,
    Int16,
    Int8,
}

impl From<ActivationDataType> for litertlm_sys::LiteRtLmActivationDataType {
    fn from(t: ActivationDataType) -> Self {
        match t {
            ActivationDataType::Float32 => {
                litertlm_sys::LiteRtLmActivationDataType_kLiteRtLmActivationDataTypeFloat32
            }
            ActivationDataType::Float16 => {
                litertlm_sys::LiteRtLmActivationDataType_kLiteRtLmActivationDataTypeFloat16
            }
            ActivationDataType::Int16 => {
                litertlm_sys::LiteRtLmActivationDataType_kLiteRtLmActivationDataTypeInt16
            }
            ActivationDataType::Int8 => {
                litertlm_sys::LiteRtLmActivationDataType_kLiteRtLmActivationDataTypeInt8
            }
        }
    }
}

/// Wraps `LiteRtLmTokenUnionType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenUnionType {
    String,
    Ids,
}

impl From<litertlm_sys::LiteRtLmTokenUnionType> for TokenUnionType {
    fn from(raw: litertlm_sys::LiteRtLmTokenUnionType) -> Self {
        #[allow(non_upper_case_globals)]
        match raw {
            litertlm_sys::LiteRtLmTokenUnionType_kLiteRtLmTokenUnionTypeIds => TokenUnionType::Ids,
            _ => TokenUnionType::String,
        }
    }
}

// =======================================================================
// SamplerParams
// =======================================================================

opaque_owned!(
    SamplerParams,
    LiteRtLmSamplerParams,
    litertlm_sys::litert_lm_sampler_params_delete
);

impl SamplerParams {
    /// Wraps `litert_lm_sampler_params_create`.
    pub fn new(sampler_type: SamplerType) -> Result<Self, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_sampler_params_create(sampler_type.into()) };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_sampler_params_create")?,
        })
    }

    /// Wraps `litert_lm_sampler_params_set_top_k`.
    pub fn set_top_k(&mut self, top_k: i32) -> &mut Self {
        unsafe { litertlm_sys::litert_lm_sampler_params_set_top_k(self.ptr, top_k) };
        self
    }

    /// Wraps `litert_lm_sampler_params_set_top_p`.
    pub fn set_top_p(&mut self, top_p: f32) -> &mut Self {
        unsafe { litertlm_sys::litert_lm_sampler_params_set_top_p(self.ptr, top_p) };
        self
    }

    /// Wraps `litert_lm_sampler_params_set_temperature`.
    pub fn set_temperature(&mut self, temperature: f32) -> &mut Self {
        unsafe { litertlm_sys::litert_lm_sampler_params_set_temperature(self.ptr, temperature) };
        self
    }

    /// Wraps `litert_lm_sampler_params_set_seed`.
    pub fn set_seed(&mut self, seed: i32) -> &mut Self {
        unsafe { litertlm_sys::litert_lm_sampler_params_set_seed(self.ptr, seed) };
        self
    }
}

// =======================================================================
// RepetitionPenaltyConfig
// =======================================================================

opaque_owned!(
    RepetitionPenaltyConfig,
    LiteRtLmRepetitionPenaltyConfig,
    litertlm_sys::litert_lm_repetition_penalty_config_delete
);

impl RepetitionPenaltyConfig {
    /// Wraps `litert_lm_repetition_penalty_config_create`. Defaults:
    /// `repetition_penalty` = 1.0, `presence_penalty` = 0.0,
    /// `frequency_penalty` = 0.0, `window_size` = 0 (all history, no
    /// penalties active).
    pub fn new() -> Result<Self, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_repetition_penalty_config_create() };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_repetition_penalty_config_create")?,
        })
    }

    /// Wraps `litert_lm_repetition_penalty_config_set_repetition_penalty`.
    pub fn set_repetition_penalty(&mut self, value: f32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_repetition_penalty_config_set_repetition_penalty(
                self.ptr, value,
            )
        };
        self
    }

    /// Wraps `litert_lm_repetition_penalty_config_set_presence_penalty`.
    pub fn set_presence_penalty(&mut self, value: f32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_repetition_penalty_config_set_presence_penalty(self.ptr, value)
        };
        self
    }

    /// Wraps `litert_lm_repetition_penalty_config_set_frequency_penalty`.
    pub fn set_frequency_penalty(&mut self, value: f32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_repetition_penalty_config_set_frequency_penalty(self.ptr, value)
        };
        self
    }

    /// Wraps `litert_lm_repetition_penalty_config_set_window_size`.
    pub fn set_window_size(&mut self, window_size: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_repetition_penalty_config_set_window_size(
                self.ptr,
                window_size,
            )
        };
        self
    }
}

// =======================================================================
// NoRepeatNgramConfig
// =======================================================================

opaque_owned!(
    NoRepeatNgramConfig,
    LiteRtLmNoRepeatNgramConfig,
    litertlm_sys::litert_lm_no_repeat_ngram_config_delete
);

impl NoRepeatNgramConfig {
    /// Wraps `litert_lm_no_repeat_ngram_config_create`. Defaults: size = 0,
    /// window = 0 (banning disabled).
    pub fn new() -> Result<Self, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_no_repeat_ngram_config_create() };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_no_repeat_ngram_config_create")?,
        })
    }

    /// Wraps `litert_lm_no_repeat_ngram_config_set_no_repeat_ngram_size`.
    pub fn set_no_repeat_ngram_size(&mut self, size: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_no_repeat_ngram_config_set_no_repeat_ngram_size(
                self.ptr, size,
            )
        };
        self
    }

    /// Wraps `litert_lm_no_repeat_ngram_config_set_window_size`.
    pub fn set_window_size(&mut self, window_size: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_no_repeat_ngram_config_set_window_size(self.ptr, window_size)
        };
        self
    }
}

// =======================================================================
// SuppressTokensConfig
// =======================================================================

opaque_owned!(
    SuppressTokensConfig,
    LiteRtLmSuppressTokensConfig,
    litertlm_sys::litert_lm_suppress_tokens_config_delete
);

impl SuppressTokensConfig {
    /// Wraps `litert_lm_suppress_tokens_config_create`. Defaults: empty set
    /// (suppression disabled).
    pub fn new() -> Result<Self, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_suppress_tokens_config_create() };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_suppress_tokens_config_create")?,
        })
    }

    /// Wraps `litert_lm_suppress_tokens_config_set_suppress_tokens`. Pass an
    /// empty slice to clear/disable suppression.
    pub fn set_suppress_tokens(&mut self, tokens: &[i32]) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_suppress_tokens_config_set_suppress_tokens(
                self.ptr,
                tokens.as_ptr(),
                tokens.len(),
            )
        };
        self
    }
}

// =======================================================================
// ThinkingConfig
// =======================================================================

opaque_owned!(
    ThinkingConfig,
    LiteRtLmThinkingConfig,
    litertlm_sys::litert_lm_thinking_config_delete
);

impl ThinkingConfig {
    /// Wraps `litert_lm_thinking_config_create`. Defaults: thinking
    /// enabled, infinite budget (-1).
    pub fn new() -> Result<Self, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_thinking_config_create() };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_thinking_config_create")?,
        })
    }

    /// Wraps `litert_lm_thinking_config_set_enable_thinking`.
    pub fn set_enable_thinking(&mut self, enable: bool) -> &mut Self {
        unsafe { litertlm_sys::litert_lm_thinking_config_set_enable_thinking(self.ptr, enable) };
        self
    }

    /// Wraps `litert_lm_thinking_config_set_thinking_token_budget`. Pass -1
    /// for an infinite budget.
    pub fn set_thinking_token_budget(&mut self, budget: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_thinking_config_set_thinking_token_budget(self.ptr, budget)
        };
        self
    }
}

// =======================================================================
// SessionConfig
// =======================================================================

opaque_owned!(
    SessionConfig,
    LiteRtLmSessionConfig,
    litertlm_sys::litert_lm_session_config_delete
);

impl SessionConfig {
    /// Wraps `litert_lm_session_config_create`.
    pub fn new() -> Result<Self, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_session_config_create() };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_session_config_create")?,
        })
    }

    /// Wraps `litert_lm_session_config_set_max_output_tokens`.
    pub fn set_max_output_tokens(&mut self, max_output_tokens: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_session_config_set_max_output_tokens(
                self.ptr,
                max_output_tokens,
            )
        };
        self
    }

    /// Wraps `litert_lm_session_config_set_apply_prompt_template`.
    pub fn set_apply_prompt_template(&mut self, apply: bool) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_session_config_set_apply_prompt_template(self.ptr, apply)
        };
        self
    }

    /// Wraps `litert_lm_session_config_set_sampler_params`. The sampler
    /// params are copied in, so `params` can be dropped or reused
    /// afterwards.
    pub fn set_sampler_params(&mut self, params: &SamplerParams) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_session_config_set_sampler_params(self.ptr, params.as_ptr())
        };
        self
    }

    /// Wraps `litert_lm_session_config_set_lora_path`.
    pub fn set_lora_path(&mut self, lora_path: &str) -> Result<&mut Self, Error> {
        let path = cstr(lora_path)?;
        let ret =
            unsafe { litertlm_sys::litert_lm_session_config_set_lora_path(self.ptr, path.as_ptr()) };
        check_status(ret, "litert_lm_session_config_set_lora_path")?;
        Ok(self)
    }

    /// Wraps `litert_lm_session_config_set_audio_lora_path`.
    pub fn set_audio_lora_path(&mut self, audio_lora_path: &str) -> Result<&mut Self, Error> {
        let path = cstr(audio_lora_path)?;
        let ret = unsafe {
            litertlm_sys::litert_lm_session_config_set_audio_lora_path(self.ptr, path.as_ptr())
        };
        check_status(ret, "litert_lm_session_config_set_audio_lora_path")?;
        Ok(self)
    }
}

// =======================================================================
// ConversationConfig
// =======================================================================

opaque_owned!(
    ConversationConfig,
    LiteRtLmConversationConfig,
    litertlm_sys::litert_lm_conversation_config_delete
);

impl ConversationConfig {
    /// Wraps `litert_lm_conversation_config_create`.
    pub fn new() -> Result<Self, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_conversation_config_create() };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_conversation_config_create")?,
        })
    }

    /// Wraps `litert_lm_conversation_config_set_session_config`.
    pub fn set_session_config(&mut self, session_config: &SessionConfig) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_session_config(
                self.ptr,
                session_config.as_ptr(),
            )
        };
        self
    }

    /// Wraps `litert_lm_conversation_config_set_system_message`.
    pub fn set_system_message(&mut self, system_message_json: &str) -> Result<&mut Self, Error> {
        let json = cstr(system_message_json)?;
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_system_message(
                self.ptr,
                json.as_ptr(),
            )
        };
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_tools`.
    pub fn set_tools(&mut self, tools_json: &str) -> Result<&mut Self, Error> {
        let json = cstr(tools_json)?;
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_tools(self.ptr, json.as_ptr())
        };
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_messages`.
    pub fn set_messages(&mut self, messages_json: &str) -> Result<&mut Self, Error> {
        let json = cstr(messages_json)?;
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_messages(self.ptr, json.as_ptr())
        };
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_extra_context`.
    pub fn set_extra_context(&mut self, extra_context_json: &str) -> Result<&mut Self, Error> {
        let json = cstr(extra_context_json)?;
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_extra_context(
                self.ptr,
                json.as_ptr(),
            )
        };
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_prompt_template`.
    pub fn set_prompt_template(&mut self, prompt_template: &str) -> Result<&mut Self, Error> {
        let template = cstr(prompt_template)?;
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_prompt_template(
                self.ptr,
                template.as_ptr(),
            )
        };
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_enable_constrained_decoding`.
    pub fn set_enable_constrained_decoding(&mut self, enable: bool) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_enable_constrained_decoding(
                self.ptr, enable,
            )
        };
        self
    }

    /// Wraps `litert_lm_conversation_config_set_constraint_provider`. Pass
    /// `None` to unset.
    pub fn set_constraint_provider(
        &mut self,
        provider: Option<ConstraintProviderType>,
    ) -> &mut Self {
        match provider {
            Some(p) => {
                let raw: litertlm_sys::LiteRtLmConstraintProviderType = p.into();
                unsafe {
                    litertlm_sys::litert_lm_conversation_config_set_constraint_provider(
                        self.ptr, &raw,
                    )
                };
            }
            None => unsafe {
                litertlm_sys::litert_lm_conversation_config_set_constraint_provider(
                    self.ptr,
                    std::ptr::null(),
                )
            },
        }
        self
    }

    /// Wraps
    /// `litert_lm_conversation_config_set_filter_channel_content_from_kv_cache`.
    pub fn set_filter_channel_content_from_kv_cache(&mut self, filter: bool) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_filter_channel_content_from_kv_cache(
                self.ptr, filter,
            )
        };
        self
    }

    /// Wraps `litert_lm_conversation_config_set_stream_tool_calls`.
    pub fn set_stream_tool_calls(
        &mut self,
        stream: bool,
        channel_name: &str,
    ) -> Result<&mut Self, Error> {
        let name = cstr(channel_name)?;
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_stream_tool_calls(
                self.ptr,
                stream,
                name.as_ptr(),
            )
        };
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_thinking_config`.
    pub fn set_thinking_config(&mut self, thinking_config: &ThinkingConfig) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_config_set_thinking_config(
                self.ptr,
                thinking_config.as_ptr(),
            )
        };
        self
    }
}

// =======================================================================
// ConversationOptionalArgs
// =======================================================================

opaque_owned!(
    ConversationOptionalArgs,
    LiteRtLmConversationOptionalArgs,
    litertlm_sys::litert_lm_conversation_optional_args_delete
);

impl ConversationOptionalArgs {
    /// Wraps `litert_lm_conversation_optional_args_create`.
    pub fn new() -> Result<Self, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_conversation_optional_args_create() };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_conversation_optional_args_create")?,
        })
    }

    /// Wraps
    /// `litert_lm_conversation_optional_args_set_repetition_penalty_config`.
    pub fn set_repetition_penalty_config(&mut self, config: &RepetitionPenaltyConfig) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_optional_args_set_repetition_penalty_config(
                self.ptr,
                config.as_ptr(),
            )
        };
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_no_repeat_ngram_config`.
    pub fn set_no_repeat_ngram_config(&mut self, config: &NoRepeatNgramConfig) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_optional_args_set_no_repeat_ngram_config(
                self.ptr,
                config.as_ptr(),
            )
        };
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_suppress_tokens_config`.
    pub fn set_suppress_tokens_config(&mut self, config: &SuppressTokensConfig) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_optional_args_set_suppress_tokens_config(
                self.ptr,
                config.as_ptr(),
            )
        };
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_visual_token_budget`.
    pub fn set_visual_token_budget(&mut self, budget: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_optional_args_set_visual_token_budget(
                self.ptr, budget,
            )
        };
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_max_output_tokens`.
    pub fn set_max_output_tokens(&mut self, max_output_tokens: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_optional_args_set_max_output_tokens(
                self.ptr,
                max_output_tokens,
            )
        };
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_thinking_config`.
    pub fn set_thinking_config(&mut self, thinking_config: &ThinkingConfig) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_conversation_optional_args_set_thinking_config(
                self.ptr,
                thinking_config.as_ptr(),
            )
        };
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_constraint`.
    pub fn set_constraint(
        &mut self,
        constraint_type: ConstraintType,
        constraint_string: &str,
    ) -> Result<&mut Self, Error> {
        let constraint = cstr(constraint_string)?;
        unsafe {
            litertlm_sys::litert_lm_conversation_optional_args_set_constraint(
                self.ptr,
                constraint_type.into(),
                constraint.as_ptr(),
            )
        };
        Ok(self)
    }
}

// =======================================================================
// InputData
// =======================================================================

opaque_owned!(
    InputData,
    LiteRtLmInputData,
    litertlm_sys::litert_lm_input_data_delete
);

impl InputData {
    /// Wraps `litert_lm_input_data_create`. `data` is copied internally by
    /// the library. For `InputDataType::Text`, `data` should be UTF-8
    /// bytes; for image/audio types, the raw media bytes.
    pub fn new(data_type: InputDataType, data: &[u8]) -> Result<Self, Error> {
        let ptr = unsafe {
            litertlm_sys::litert_lm_input_data_create(
                data_type.into(),
                data.as_ptr() as *const c_void,
                data.len(),
            )
        };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_input_data_create")?,
        })
    }

    /// Convenience constructor for `InputDataType::Text`.
    pub fn text(text: &str) -> Result<Self, Error> {
        Self::new(InputDataType::Text, text.as_bytes())
    }
}

// =======================================================================
// EngineSettings
// =======================================================================

/// Builder for `LiteRtLmEngineSettings`.
pub struct EngineSettings {
    ptr: *mut litertlm_sys::LiteRtLmEngineSettings,
}

unsafe impl Send for EngineSettings {}

impl EngineSettings {
    /// Wraps `litert_lm_engine_settings_create`. `backend`: e.g. `"cpu"` or
    /// `"gpu"`. `vision_backend` / `audio_backend` may be left `None`.
    pub fn new(
        model_path: &str,
        backend: &str,
        vision_backend: Option<&str>,
        audio_backend: Option<&str>,
    ) -> Result<Self, Error> {
        let model_path = cstr(model_path)?;
        let backend = cstr(backend)?;
        let vision_backend = vision_backend.map(cstr).transpose()?;
        let audio_backend = audio_backend.map(cstr).transpose()?;

        let ptr = unsafe {
            litertlm_sys::litert_lm_engine_settings_create(
                model_path.as_ptr(),
                backend.as_ptr(),
                vision_backend.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                audio_backend.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            )
        };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_engine_settings_create")?,
        })
    }

    /// Wraps `litert_lm_engine_settings_create_from_raw_file_descriptor`.
    /// The engine takes ownership of `fd` and will close it when done.
    pub fn from_raw_file_descriptor(
        fd: i32,
        backend: &str,
        vision_backend: Option<&str>,
        audio_backend: Option<&str>,
    ) -> Result<Self, Error> {
        let backend = cstr(backend)?;
        let vision_backend = vision_backend.map(cstr).transpose()?;
        let audio_backend = audio_backend.map(cstr).transpose()?;

        let ptr = unsafe {
            litertlm_sys::litert_lm_engine_settings_create_from_raw_file_descriptor(
                fd,
                backend.as_ptr(),
                vision_backend.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                audio_backend.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            )
        };
        Ok(Self {
            ptr: check_created(
                ptr,
                "litert_lm_engine_settings_create_from_raw_file_descriptor",
            )?,
        })
    }

    fn as_ptr(&self) -> *const litertlm_sys::LiteRtLmEngineSettings {
        self.ptr
    }

    /// Wraps `litert_lm_engine_settings_set_max_num_tokens`.
    pub fn set_max_num_tokens(&mut self, max_num_tokens: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_max_num_tokens(self.ptr, max_num_tokens)
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_num_threads`.
    pub fn set_num_threads(&mut self, num_threads: i32) -> &mut Self {
        unsafe { litertlm_sys::litert_lm_engine_settings_set_num_threads(self.ptr, num_threads) };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_audio_num_threads`.
    pub fn set_audio_num_threads(&mut self, num_threads: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_audio_num_threads(self.ptr, num_threads)
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_parallel_file_section_loading`.
    /// Defaults to `true`.
    pub fn set_parallel_file_section_loading(&mut self, parallel: bool) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_parallel_file_section_loading(
                self.ptr, parallel,
            )
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_max_num_images`. Only used by
    /// the legacy engine implementation.
    pub fn set_max_num_images(&mut self, max_num_images: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_max_num_images(
                self.ptr,
                max_num_images,
            )
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_cache_dir`.
    pub fn set_cache_dir(&mut self, cache_dir: &str) -> Result<&mut Self, Error> {
        let cache_dir = cstr(cache_dir)?;
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_cache_dir(self.ptr, cache_dir.as_ptr())
        };
        Ok(self)
    }

    /// Wraps `litert_lm_engine_settings_set_litert_dispatch_lib_dir`.
    pub fn set_litert_dispatch_lib_dir(&mut self, lib_dir: &str) -> Result<&mut Self, Error> {
        let lib_dir = cstr(lib_dir)?;
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_litert_dispatch_lib_dir(
                self.ptr,
                lib_dir.as_ptr(),
            )
        };
        Ok(self)
    }

    /// Wraps `litert_lm_engine_settings_set_activation_data_type`.
    pub fn set_activation_data_type(&mut self, data_type: ActivationDataType) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_activation_data_type(
                self.ptr,
                data_type.into(),
            )
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_prefill_chunk_size`. Only
    /// applicable for the CPU backend with dynamic models.
    pub fn set_prefill_chunk_size(&mut self, chunk_size: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_prefill_chunk_size(self.ptr, chunk_size)
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_enable_ynnpack`. New in the
    /// LiteRT-LM v0.16.0 C API. Controls whether YNNPACK is allowed to
    /// delegate supported operations ahead of XNNPACK.
    pub fn set_enable_ynnpack(&mut self, enable: bool) -> &mut Self {
        unsafe { litertlm_sys::litert_lm_engine_settings_set_enable_ynnpack(self.ptr, enable) };
        self
    }

    /// Wraps `litert_lm_engine_settings_enable_benchmark`.
    pub fn enable_benchmark(&mut self) -> &mut Self {
        unsafe { litertlm_sys::litert_lm_engine_settings_enable_benchmark(self.ptr) };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_num_prefill_tokens`.
    pub fn set_num_prefill_tokens(&mut self, num_prefill_tokens: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_num_prefill_tokens(
                self.ptr,
                num_prefill_tokens,
            )
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_num_decode_tokens`.
    pub fn set_num_decode_tokens(&mut self, num_decode_tokens: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_num_decode_tokens(
                self.ptr,
                num_decode_tokens,
            )
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_enable_speculative_decoding`.
    pub fn set_enable_speculative_decoding(&mut self, enable: bool) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_enable_speculative_decoding(
                self.ptr, enable,
            )
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_gpu_decode_steps_per_sync`.
    /// Only supported on the Artisan GPU backend.
    pub fn set_gpu_decode_steps_per_sync(&mut self, steps: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_gpu_decode_steps_per_sync(
                self.ptr, steps,
            )
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_gpu_wait_for_weight_uploads`.
    /// Only supported on the Artisan GPU backend.
    pub fn set_gpu_wait_for_weight_uploads(&mut self, wait: bool) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_gpu_wait_for_weight_uploads(
                self.ptr, wait,
            )
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_use_ringbuffers_local_attention`.
    /// Currently only honored on the GPU Artisan backend; ignored (with a
    /// warning from the underlying library) elsewhere.
    pub fn set_use_ringbuffers_local_attention(&mut self, use_ringbuffers: bool) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_use_ringbuffers_local_attention(
                self.ptr,
                use_ringbuffers,
            )
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_lora_rank`.
    pub fn set_lora_rank(&mut self, lora_rank: i32) -> &mut Self {
        unsafe { litertlm_sys::litert_lm_engine_settings_set_lora_rank(self.ptr, lora_rank) };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_supported_lora_ranks`.
    pub fn set_supported_lora_ranks(&mut self, ranks: &[i32]) -> Result<&mut Self, Error> {
        let ret = unsafe {
            litertlm_sys::litert_lm_engine_settings_set_supported_lora_ranks(
                self.ptr,
                ranks.as_ptr(),
                ranks.len(),
            )
        };
        check_status(ret, "litert_lm_engine_settings_set_supported_lora_ranks")?;
        Ok(self)
    }

    /// Wraps `litert_lm_engine_settings_set_audio_lora_rank`.
    pub fn set_audio_lora_rank(&mut self, lora_rank: i32) -> &mut Self {
        unsafe {
            litertlm_sys::litert_lm_engine_settings_set_audio_lora_rank(self.ptr, lora_rank)
        };
        self
    }

    /// Wraps `litert_lm_engine_settings_set_supported_audio_lora_ranks`.
    pub fn set_supported_audio_lora_ranks(&mut self, ranks: &[i32]) -> Result<&mut Self, Error> {
        let ret = unsafe {
            litertlm_sys::litert_lm_engine_settings_set_supported_audio_lora_ranks(
                self.ptr,
                ranks.as_ptr(),
                ranks.len(),
            )
        };
        check_status(
            ret,
            "litert_lm_engine_settings_set_supported_audio_lora_ranks",
        )?;
        Ok(self)
    }
}

impl Drop for EngineSettings {
    fn drop(&mut self) {
        unsafe { litertlm_sys::litert_lm_engine_settings_delete(self.ptr) };
    }
}

// =======================================================================
// Engine
// =======================================================================

pub struct Engine {
    ptr: *mut litertlm_sys::LiteRtLmEngine,
}

unsafe impl Send for Engine {}

impl Engine {
    /// Wraps `litert_lm_engine_create`.
    pub fn new(settings: &EngineSettings) -> Result<Self, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_engine_create(settings.as_ptr()) };
        Ok(Self {
            ptr: check_created(ptr, "litert_lm_engine_create")?,
        })
    }

    /// Wraps `litert_lm_engine_create_session`. Pass `None` to use the
    /// default session config.
    pub fn create_session(&self, config: Option<&SessionConfig>) -> Result<Session, Error> {
        let config_ptr = config.map_or(std::ptr::null_mut(), |c| c.ptr);
        let ptr = unsafe { litertlm_sys::litert_lm_engine_create_session(self.ptr, config_ptr) };
        Ok(Session {
            ptr: check_created(ptr, "litert_lm_engine_create_session")?,
        })
    }

    /// Wraps `litert_lm_conversation_create` with the default conversation
    /// config. See [`Engine::create_conversation_with_config`] to customize
    /// system message, tools, thinking, etc.
    pub fn create_conversation(&self) -> Result<Conversation, Error> {
        let ptr = unsafe {
            litertlm_sys::litert_lm_conversation_create(self.ptr, std::ptr::null_mut())
        };
        Ok(Conversation {
            ptr: check_created(ptr, "litert_lm_conversation_create")?,
        })
    }

    /// Wraps `litert_lm_conversation_create` with an explicit
    /// [`ConversationConfig`].
    pub fn create_conversation_with_config(
        &self,
        config: &ConversationConfig,
    ) -> Result<Conversation, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_conversation_create(self.ptr, config.ptr) };
        Ok(Conversation {
            ptr: check_created(ptr, "litert_lm_conversation_create")?,
        })
    }

    /// Wraps `litert_lm_engine_tokenize`.
    pub fn tokenize(&self, text: &str) -> Result<TokenizeResult, Error> {
        let text_c = cstr(text)?;
        let ptr = unsafe { litertlm_sys::litert_lm_engine_tokenize(self.ptr, text_c.as_ptr()) };
        Ok(TokenizeResult {
            ptr: check_created(ptr, "litert_lm_engine_tokenize")?,
        })
    }

    /// Wraps `litert_lm_engine_detokenize`.
    pub fn detokenize(&self, tokens: &[i32]) -> Result<DetokenizeResult, Error> {
        let ptr = unsafe {
            litertlm_sys::litert_lm_engine_detokenize(self.ptr, tokens.as_ptr(), tokens.len())
        };
        Ok(DetokenizeResult {
            ptr: check_created(ptr, "litert_lm_engine_detokenize")?,
        })
    }

    /// Wraps `litert_lm_engine_get_start_token`. Returns `None` if no start
    /// (BOS) token is configured.
    pub fn get_start_token(&self) -> Option<TokenUnion> {
        let ptr = unsafe { litertlm_sys::litert_lm_engine_get_start_token(self.ptr) };
        if ptr.is_null() {
            None
        } else {
            Some(TokenUnion { ptr })
        }
    }

    /// Wraps `litert_lm_engine_get_stop_tokens`. Returns `None` if no stop
    /// (EOS) tokens are configured.
    pub fn get_stop_tokens(&self) -> Option<TokenUnions> {
        let ptr = unsafe { litertlm_sys::litert_lm_engine_get_stop_tokens(self.ptr) };
        if ptr.is_null() {
            None
        } else {
            Some(TokenUnions { ptr })
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe { litertlm_sys::litert_lm_engine_delete(self.ptr) };
    }
}

// =======================================================================
// Streaming (shared by Conversation and Session)
// =======================================================================

/// Borrowed view of a single streamed chunk. Only valid for the duration of
/// the callback that receives it -- do not store it.
pub struct StreamChunk {
    ptr: *const litertlm_sys::LiteRtLmStreamChunk,
}

impl StreamChunk {
    /// Wraps `litert_lm_stream_chunk_get_text`.
    pub fn text(&self) -> Option<String> {
        unsafe { owned_str_from(litertlm_sys::litert_lm_stream_chunk_get_text(self.ptr)) }
    }

    /// Wraps `litert_lm_stream_chunk_is_final`.
    pub fn is_final(&self) -> bool {
        unsafe { litertlm_sys::litert_lm_stream_chunk_is_final(self.ptr) }
    }

    /// Wraps `litert_lm_stream_chunk_get_error`.
    pub fn error(&self) -> Option<String> {
        unsafe { owned_str_from(litertlm_sys::litert_lm_stream_chunk_get_error(self.ptr)) }
    }
}

/// Shared implementation behind `Conversation::send_message_stream`,
/// `Session::run_decode_async`, and `Session::generate_content_stream`.
///
/// The underlying C functions are all asynchronous -- they return
/// immediately and invoke the callback from a background thread. To keep
/// this binding safe, this blocks until the final chunk (or an error) is
/// observed, so `on_chunk` and everything it captures only need to stay
/// alive for the duration of the call, not beyond it. `call` performs the
/// actual FFI call, given the trampoline function pointer and opaque data
/// pointer to pass as the C callback + callback_data pair.
fn run_stream<F: FnMut(&StreamChunk)>(
    on_chunk: F,
    call: impl FnOnce(litertlm_sys::LiteRtLmStreamCallback, *mut c_void) -> c_int,
    what: &'static str,
) -> Result<(), Error> {
    struct StreamState<'a, F: FnMut(&StreamChunk)> {
        callback: &'a mut F,
        done_tx: std::sync::mpsc::Sender<Result<(), String>>,
    }

    extern "C" fn trampoline<F: FnMut(&StreamChunk)>(
        callback_data: *mut c_void,
        chunk: *const litertlm_sys::LiteRtLmStreamChunk,
    ) {
        // SAFETY: `callback_data` points at a `StreamState<F>` owned by the
        // stack frame of `run_stream` below, which does not return until
        // this trampoline has signalled `done_tx` for the final chunk -- so
        // the pointee is guaranteed alive here.
        let state = unsafe { &mut *(callback_data as *mut StreamState<F>) };
        let chunk = StreamChunk { ptr: chunk };
        let is_final = chunk.is_final();
        let error = chunk.error();
        (state.callback)(&chunk);
        if is_final {
            let result = match error {
                Some(msg) => Err(msg),
                None => Ok(()),
            };
            let _ = state.done_tx.send(result);
        }
    }

    let mut on_chunk = on_chunk;
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let mut state = StreamState {
        callback: &mut on_chunk,
        done_tx,
    };

    let ret = call(
        Some(trampoline::<F>),
        &mut state as *mut StreamState<F> as *mut c_void,
    );
    check_status(ret, what)?;

    match done_rx.recv() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(msg)) => Err(Error::Stream(msg)),
        Err(_) => Err(Error::Stream(
            "stream channel closed before a final chunk arrived".into(),
        )),
    }
}

// =======================================================================
// Session
// =======================================================================

pub struct Session {
    ptr: *mut litertlm_sys::LiteRtLmSession,
}

unsafe impl Send for Session {}

impl Session {
    /// Wraps `litert_lm_session_cancel_process`.
    pub fn cancel_process(&self) {
        unsafe { litertlm_sys::litert_lm_session_cancel_process(self.ptr) };
    }

    /// Wraps `litert_lm_session_save_checkpoint`. New in the LiteRT-LM
    /// v0.16.0 C API. Saves the session's current state under `label`, so
    /// you can later return to it with [`Session::rewind_to_checkpoint`].
    pub fn save_checkpoint(&self, label: &str) -> Result<(), Error> {
        let label_c = cstr(label)?;
        let ret = unsafe {
            litertlm_sys::litert_lm_session_save_checkpoint(self.ptr, label_c.as_ptr())
        };
        check_status(ret, "litert_lm_session_save_checkpoint")
    }

    /// Wraps `litert_lm_session_rewind_to_checkpoint`. New in the
    /// LiteRT-LM v0.16.0 C API.
    pub fn rewind_to_checkpoint(&self, label: &str) -> Result<(), Error> {
        let label_c = cstr(label)?;
        let ret = unsafe {
            litertlm_sys::litert_lm_session_rewind_to_checkpoint(self.ptr, label_c.as_ptr())
        };
        check_status(ret, "litert_lm_session_rewind_to_checkpoint")
    }

    /// Wraps `litert_lm_session_rewind_to_step`. New in the LiteRT-LM
    /// v0.16.0 C API.
    pub fn rewind_to_step(&self, step: i32) -> Result<(), Error> {
        let ret = unsafe { litertlm_sys::litert_lm_session_rewind_to_step(self.ptr, step) };
        check_status(ret, "litert_lm_session_rewind_to_step")
    }

    /// Wraps `litert_lm_session_run_prefill`. Blocks until prefill
    /// completes.
    pub fn run_prefill(&self, inputs: &[InputData]) -> Result<(), Error> {
        let ptrs: Vec<*const litertlm_sys::LiteRtLmInputData> =
            inputs.iter().map(|i| i.as_ptr()).collect();
        let ret = unsafe {
            litertlm_sys::litert_lm_session_run_prefill(self.ptr, ptrs.as_ptr(), ptrs.len())
        };
        check_status(ret, "litert_lm_session_run_prefill")
    }

    /// Wraps `litert_lm_session_run_decode`. Blocks until decoding
    /// completes; call after [`Session::run_prefill`].
    pub fn run_decode(&self) -> Result<Responses, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_session_run_decode(self.ptr) };
        Ok(Responses {
            ptr: check_created(ptr, "litert_lm_session_run_decode")?,
        })
    }

    /// Wraps `litert_lm_session_run_text_scoring`. Scores the given target
    /// texts after prefill.
    pub fn run_text_scoring(
        &self,
        target_texts: &[&str],
        store_token_lengths: bool,
    ) -> Result<Responses, Error> {
        let c_strings: Vec<CString> = target_texts
            .iter()
            .map(|s| cstr(s))
            .collect::<Result<_, _>>()?;
        let mut ptrs: Vec<*const c_char> = c_strings.iter().map(|s| s.as_ptr()).collect();
        let ptr = unsafe {
            litertlm_sys::litert_lm_session_run_text_scoring(
                self.ptr,
                ptrs.as_mut_ptr(),
                ptrs.len(),
                store_token_lengths,
            )
        };
        Ok(Responses {
            ptr: check_created(ptr, "litert_lm_session_run_text_scoring")?,
        })
    }

    /// Wraps `litert_lm_session_generate_content`. Combines prefill +
    /// decode into one blocking call.
    pub fn generate_content(&self, inputs: &[InputData]) -> Result<Responses, Error> {
        let ptrs: Vec<*const litertlm_sys::LiteRtLmInputData> =
            inputs.iter().map(|i| i.as_ptr()).collect();
        let ptr = unsafe {
            litertlm_sys::litert_lm_session_generate_content(
                self.ptr,
                ptrs.as_ptr(),
                ptrs.len(),
            )
        };
        Ok(Responses {
            ptr: check_created(ptr, "litert_lm_session_generate_content")?,
        })
    }

    /// Wraps `litert_lm_session_run_decode_async`, blocking internally
    /// until the stream completes (see [`run_stream`] for why). Call after
    /// [`Session::run_prefill`].
    pub fn run_decode_async<F: FnMut(&StreamChunk)>(&self, on_chunk: F) -> Result<(), Error> {
        run_stream(
            on_chunk,
            |cb, data| unsafe {
                litertlm_sys::litert_lm_session_run_decode_async(self.ptr, cb, data)
            },
            "litert_lm_session_run_decode_async",
        )
    }

    /// Wraps `litert_lm_session_generate_content_stream`, blocking
    /// internally until the stream completes (see [`run_stream`] for why).
    pub fn generate_content_stream<F: FnMut(&StreamChunk)>(
        &self,
        inputs: &[InputData],
        on_chunk: F,
    ) -> Result<(), Error> {
        let ptrs: Vec<*const litertlm_sys::LiteRtLmInputData> =
            inputs.iter().map(|i| i.as_ptr()).collect();
        run_stream(
            on_chunk,
            |cb, data| unsafe {
                litertlm_sys::litert_lm_session_generate_content_stream(
                    self.ptr,
                    ptrs.as_ptr(),
                    ptrs.len(),
                    cb,
                    data,
                )
            },
            "litert_lm_session_generate_content_stream",
        )
    }

    /// Wraps `litert_lm_session_get_benchmark_info`. Requires
    /// [`EngineSettings::enable_benchmark`] to have been called before the
    /// engine was created.
    pub fn get_benchmark_info(&self) -> Result<BenchmarkInfo, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_session_get_benchmark_info(self.ptr) };
        Ok(BenchmarkInfo {
            ptr: check_created(ptr, "litert_lm_session_get_benchmark_info")?,
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe { litertlm_sys::litert_lm_session_delete(self.ptr) };
    }
}

// =======================================================================
// Responses
// =======================================================================

opaque_owned!(
    Responses,
    LiteRtLmResponses,
    litertlm_sys::litert_lm_responses_delete
);

impl Responses {
    /// Wraps `litert_lm_responses_get_num_candidates`.
    pub fn num_candidates(&self) -> i32 {
        unsafe { litertlm_sys::litert_lm_responses_get_num_candidates(self.ptr) }
    }

    /// Wraps `litert_lm_responses_get_response_text_at`.
    pub fn response_text_at(&self, index: i32) -> Option<String> {
        unsafe {
            owned_str_from(litertlm_sys::litert_lm_responses_get_response_text_at(
                self.ptr, index,
            ))
        }
    }

    /// Wraps `litert_lm_responses_has_score_at` +
    /// `litert_lm_responses_get_score_at`.
    pub fn score_at(&self, index: i32) -> Option<f32> {
        unsafe {
            if litertlm_sys::litert_lm_responses_has_score_at(self.ptr, index) {
                Some(litertlm_sys::litert_lm_responses_get_score_at(
                    self.ptr, index,
                ))
            } else {
                None
            }
        }
    }

    /// Wraps `litert_lm_responses_has_token_length_at` +
    /// `litert_lm_responses_get_token_length_at`.
    pub fn token_length_at(&self, index: i32) -> Option<i32> {
        unsafe {
            if litertlm_sys::litert_lm_responses_has_token_length_at(self.ptr, index) {
                Some(litertlm_sys::litert_lm_responses_get_token_length_at(
                    self.ptr, index,
                ))
            } else {
                None
            }
        }
    }

    /// Wraps `litert_lm_responses_has_token_scores_at` +
    /// `litert_lm_responses_get_num_token_scores_at` +
    /// `litert_lm_responses_get_token_scores_at`, copied into an owned
    /// `Vec` (the raw pointer is only valid for the `Responses` object's
    /// lifetime).
    pub fn token_scores_at(&self, index: i32) -> Option<Vec<f32>> {
        unsafe {
            if !litertlm_sys::litert_lm_responses_has_token_scores_at(self.ptr, index) {
                return None;
            }
            let num = litertlm_sys::litert_lm_responses_get_num_token_scores_at(self.ptr, index);
            let scores_ptr = litertlm_sys::litert_lm_responses_get_token_scores_at(self.ptr, index);
            if scores_ptr.is_null() || num <= 0 {
                return None;
            }
            Some(std::slice::from_raw_parts(scores_ptr, num as usize).to_vec())
        }
    }
}

// =======================================================================
// BenchmarkInfo
// =======================================================================

opaque_owned!(
    BenchmarkInfo,
    LiteRtLmBenchmarkInfo,
    litertlm_sys::litert_lm_benchmark_info_delete
);

impl BenchmarkInfo {
    /// Wraps `litert_lm_benchmark_info_get_time_to_first_token`. Seconds;
    /// excludes initialization time.
    pub fn time_to_first_token(&self) -> f64 {
        unsafe { litertlm_sys::litert_lm_benchmark_info_get_time_to_first_token(self.ptr) }
    }

    /// Wraps `litert_lm_benchmark_info_get_total_init_time_in_second`.
    pub fn total_init_time_in_second(&self) -> f64 {
        unsafe {
            litertlm_sys::litert_lm_benchmark_info_get_total_init_time_in_second(self.ptr)
        }
    }

    /// Wraps `litert_lm_benchmark_info_get_num_prefill_turns`.
    pub fn num_prefill_turns(&self) -> i32 {
        unsafe { litertlm_sys::litert_lm_benchmark_info_get_num_prefill_turns(self.ptr) }
    }

    /// Wraps `litert_lm_benchmark_info_get_num_decode_turns`.
    pub fn num_decode_turns(&self) -> i32 {
        unsafe { litertlm_sys::litert_lm_benchmark_info_get_num_decode_turns(self.ptr) }
    }

    /// Wraps `litert_lm_benchmark_info_get_prefill_token_count_at`.
    pub fn prefill_token_count_at(&self, index: i32) -> i32 {
        unsafe {
            litertlm_sys::litert_lm_benchmark_info_get_prefill_token_count_at(self.ptr, index)
        }
    }

    /// Wraps `litert_lm_benchmark_info_get_decode_token_count_at`.
    pub fn decode_token_count_at(&self, index: i32) -> i32 {
        unsafe {
            litertlm_sys::litert_lm_benchmark_info_get_decode_token_count_at(self.ptr, index)
        }
    }

    /// Wraps `litert_lm_benchmark_info_get_prefill_tokens_per_sec_at`.
    pub fn prefill_tokens_per_sec_at(&self, index: i32) -> f64 {
        unsafe {
            litertlm_sys::litert_lm_benchmark_info_get_prefill_tokens_per_sec_at(self.ptr, index)
        }
    }

    /// Wraps `litert_lm_benchmark_info_get_decode_tokens_per_sec_at`.
    pub fn decode_tokens_per_sec_at(&self, index: i32) -> f64 {
        unsafe {
            litertlm_sys::litert_lm_benchmark_info_get_decode_tokens_per_sec_at(self.ptr, index)
        }
    }
}

// =======================================================================
// Conversation
// =======================================================================

pub struct Conversation {
    ptr: *mut litertlm_sys::LiteRtLmConversation,
}

unsafe impl Send for Conversation {}

impl Conversation {
    /// Wraps `litert_lm_conversation_clone`, duplicating the prefilled
    /// state.
    pub fn clone_conversation(&self) -> Result<Conversation, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_conversation_clone(self.ptr) };
        Ok(Conversation {
            ptr: check_created(ptr, "litert_lm_conversation_clone")?,
        })
    }

    /// Sends a plain-text user message and blocks until the full response
    /// comes back. `message_json` follows the library's message JSON
    /// schema, e.g. `{"role": "user", "content": "hello"}`. Equivalent to
    /// `send_message_with_args(message_json, None, None)`.
    pub fn send_message(&self, message_json: &str) -> Result<String, Error> {
        self.send_message_with_args(message_json, None, None)
    }

    /// Wraps `litert_lm_conversation_send_message` with optional extra
    /// context and per-turn [`ConversationOptionalArgs`].
    pub fn send_message_with_args(
        &self,
        message_json: &str,
        extra_context: Option<&str>,
        optional_args: Option<&ConversationOptionalArgs>,
    ) -> Result<String, Error> {
        let message_c = cstr(message_json)?;
        let extra_context_c = extra_context.map(cstr).transpose()?;
        let response_ptr = unsafe {
            litertlm_sys::litert_lm_conversation_send_message(
                self.ptr,
                message_c.as_ptr(),
                extra_context_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                optional_args.map_or(std::ptr::null(), |a| a.as_ptr()),
            )
        };
        let response_ptr = check_created(response_ptr, "litert_lm_conversation_send_message")?;
        let result = unsafe {
            owned_str_from(litertlm_sys::litert_lm_json_response_get_string(
                response_ptr,
            ))
        };
        unsafe { litertlm_sys::litert_lm_json_response_delete(response_ptr) };
        result.ok_or(Error::CreateFailed("litert_lm_json_response_get_string"))
    }

    /// Sends a message and streams the response, calling `on_chunk` for
    /// every chunk as it arrives. Equivalent to
    /// `send_message_stream_with_args(message_json, None, None, on_chunk)`.
    ///
    /// See [`run_stream`] for why this blocks internally until the stream
    /// completes.
    pub fn send_message_stream<F: FnMut(&StreamChunk)>(
        &self,
        message_json: &str,
        on_chunk: F,
    ) -> Result<(), Error> {
        self.send_message_stream_with_args(message_json, None, None, on_chunk)
    }

    /// Wraps `litert_lm_conversation_send_message_stream` with optional
    /// extra context and per-turn [`ConversationOptionalArgs`].
    pub fn send_message_stream_with_args<F: FnMut(&StreamChunk)>(
        &self,
        message_json: &str,
        extra_context: Option<&str>,
        optional_args: Option<&ConversationOptionalArgs>,
        on_chunk: F,
    ) -> Result<(), Error> {
        let message_c = cstr(message_json)?;
        let extra_context_c = extra_context.map(cstr).transpose()?;
        let optional_args_ptr = optional_args.map_or(std::ptr::null(), |a| a.as_ptr());
        run_stream(
            on_chunk,
            |cb, data| unsafe {
                litertlm_sys::litert_lm_conversation_send_message_stream(
                    self.ptr,
                    message_c.as_ptr(),
                    extra_context_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                    optional_args_ptr,
                    cb,
                    data,
                )
            },
            "litert_lm_conversation_send_message_stream",
        )
    }

    /// Wraps `litert_lm_conversation_render_message_to_string`. This does
    /// NOT need to be called for normal message sending -- `send_message`
    /// handles rendering internally. Copies the result into an owned
    /// `String` immediately, since the C string is only valid until the
    /// next call to this function or conversation deletion.
    pub fn render_message_to_string(&self, message_json: &str) -> Result<String, Error> {
        let message_c = cstr(message_json)?;
        let ptr = unsafe {
            litertlm_sys::litert_lm_conversation_render_message_to_string(
                self.ptr,
                message_c.as_ptr(),
            )
        };
        unsafe { owned_str_from(ptr) }
            .ok_or(Error::CreateFailed("litert_lm_conversation_render_message_to_string"))
    }

    /// Wraps `litert_lm_conversation_render_preface_to_string`.
    pub fn render_preface_to_string(&self) -> Result<String, Error> {
        let ptr =
            unsafe { litertlm_sys::litert_lm_conversation_render_preface_to_string(self.ptr) };
        unsafe { owned_str_from(ptr) }
            .ok_or(Error::CreateFailed("litert_lm_conversation_render_preface_to_string"))
    }

    /// Wraps `litert_lm_conversation_cancel_process`.
    pub fn cancel_process(&self) {
        unsafe { litertlm_sys::litert_lm_conversation_cancel_process(self.ptr) };
    }

    /// Wraps `litert_lm_conversation_get_benchmark_info`. Requires
    /// [`EngineSettings::enable_benchmark`] to have been called before the
    /// engine was created.
    pub fn get_benchmark_info(&self) -> Result<BenchmarkInfo, Error> {
        let ptr = unsafe { litertlm_sys::litert_lm_conversation_get_benchmark_info(self.ptr) };
        Ok(BenchmarkInfo {
            ptr: check_created(ptr, "litert_lm_conversation_get_benchmark_info")?,
        })
    }

    /// Wraps `litert_lm_conversation_get_token_count`: the number of
    /// tokens in the conversation's KV cache (prefill + decode).
    pub fn token_count(&self) -> Result<i32, Error> {
        let count = unsafe { litertlm_sys::litert_lm_conversation_get_token_count(self.ptr) };
        if count < 0 {
            Err(Error::CallFailed("litert_lm_conversation_get_token_count"))
        } else {
            Ok(count)
        }
    }
}

impl Drop for Conversation {
    fn drop(&mut self) {
        unsafe { litertlm_sys::litert_lm_conversation_delete(self.ptr) };
    }
}

// =======================================================================
// TokenizeResult / DetokenizeResult
// =======================================================================

opaque_owned!(
    TokenizeResult,
    LiteRtLmTokenizeResult,
    litertlm_sys::litert_lm_tokenize_result_delete
);

impl TokenizeResult {
    /// Wraps `litert_lm_tokenize_result_get_tokens` +
    /// `litert_lm_tokenize_result_get_num_tokens`, copied into an owned
    /// `Vec`.
    pub fn tokens(&self) -> Vec<i32> {
        unsafe {
            let num = litertlm_sys::litert_lm_tokenize_result_get_num_tokens(self.ptr);
            let ptr = litertlm_sys::litert_lm_tokenize_result_get_tokens(self.ptr);
            if ptr.is_null() || num == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(ptr, num).to_vec()
            }
        }
    }
}

opaque_owned!(
    DetokenizeResult,
    LiteRtLmDetokenizeResult,
    litertlm_sys::litert_lm_detokenize_result_delete
);

impl DetokenizeResult {
    /// Wraps `litert_lm_detokenize_result_get_string`.
    pub fn text(&self) -> Option<String> {
        unsafe {
            owned_str_from(litertlm_sys::litert_lm_detokenize_result_get_string(
                self.ptr,
            ))
        }
    }
}

// =======================================================================
// TokenUnion / TokenUnions
// =======================================================================

opaque_owned!(
    TokenUnion,
    LiteRtLmTokenUnion,
    litertlm_sys::litert_lm_token_union_delete
);

impl TokenUnion {
    /// Wraps `litert_lm_token_union_get_type`.
    pub fn union_type(&self) -> TokenUnionType {
        unsafe { litertlm_sys::litert_lm_token_union_get_type(self.ptr).into() }
    }

    /// Wraps `litert_lm_token_union_get_string`. Returns `None` if this
    /// token union holds token ids instead of a string.
    pub fn as_string(&self) -> Option<String> {
        unsafe { owned_str_from(litertlm_sys::litert_lm_token_union_get_string(self.ptr)) }
    }

    /// Wraps `litert_lm_token_union_get_ids`, copied into an owned `Vec`.
    /// Returns `None` if this token union holds a string instead of ids.
    pub fn as_ids(&self) -> Option<Vec<i32>> {
        unsafe {
            let mut out_tokens: *const c_int = std::ptr::null();
            let mut out_num_tokens: usize = 0;
            let ret = litertlm_sys::litert_lm_token_union_get_ids(
                self.ptr,
                &mut out_tokens,
                &mut out_num_tokens,
            );
            if ret != 0 || out_tokens.is_null() {
                None
            } else {
                Some(std::slice::from_raw_parts(out_tokens, out_num_tokens).to_vec())
            }
        }
    }
}

opaque_owned!(
    TokenUnions,
    LiteRtLmTokenUnions,
    litertlm_sys::litert_lm_token_unions_delete
);

impl TokenUnions {
    /// Wraps `litert_lm_token_unions_get_num_tokens`.
    pub fn len(&self) -> usize {
        unsafe { litertlm_sys::litert_lm_token_unions_get_num_tokens(self.ptr) }
    }

    /// Returns `true` if this collection has no token unions.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Wraps `litert_lm_token_unions_get_token_at`. Returns an owned,
    /// independently-droppable [`TokenUnion`].
    pub fn get(&self, index: usize) -> Option<TokenUnion> {
        let ptr = unsafe { litertlm_sys::litert_lm_token_unions_get_token_at(self.ptr, index) };
        if ptr.is_null() {
            None
        } else {
            Some(TokenUnion { ptr })
        }
    }
}

//! Configuration objects: sampler, decoding controls, session and
//! conversation configs, per-turn optional args, and input data.
//!
//! Setters that can only fail on a NULL pointer (impossible with these
//! types) return `&mut Self` so they can be chained. Setters that can fail
//! for real reasons (e.g. a LoRA file that cannot be opened) return
//! `Result<&mut Self>`.

use std::ffi::c_void;

use litertlm_sys as sys;

use crate::error::Result;
use crate::types::*;
use crate::util::*;

// =======================================================================
// SamplerParams
// =======================================================================

owned_handle!(
    /// Sampling parameters. Wraps `LiteRtLmSamplerParams`.
    SamplerParams,
    LiteRtLmSamplerParams,
    litert_lm_sampler_params_delete
);

impl SamplerParams {
    /// Wraps `litert_lm_sampler_params_create`.
    pub fn new(sampler_type: SamplerType) -> Result<Self> {
        let ptr = create("litert_lm_sampler_params_create", |out| unsafe {
            sys::litert_lm_sampler_params_create(sampler_type.into(), out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_sampler_params_set_top_k`.
    pub fn set_top_k(&mut self, top_k: i32) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_sampler_params_set_top_k(self.as_ptr(), top_k) },
            "litert_lm_sampler_params_set_top_k",
        );
        self
    }

    /// Wraps `litert_lm_sampler_params_set_top_p`.
    pub fn set_top_p(&mut self, top_p: f32) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_sampler_params_set_top_p(self.as_ptr(), top_p) },
            "litert_lm_sampler_params_set_top_p",
        );
        self
    }

    /// Wraps `litert_lm_sampler_params_set_temperature`.
    pub fn set_temperature(&mut self, temperature: f32) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_sampler_params_set_temperature(self.as_ptr(), temperature) },
            "litert_lm_sampler_params_set_temperature",
        );
        self
    }

    /// Wraps `litert_lm_sampler_params_set_seed`.
    pub fn set_seed(&mut self, seed: i32) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_sampler_params_set_seed(self.as_ptr(), seed) },
            "litert_lm_sampler_params_set_seed",
        );
        self
    }
}

// =======================================================================
// RepetitionPenaltyConfig
// =======================================================================

owned_handle!(
    /// Wraps `LiteRtLmRepetitionPenaltyConfig`.
    RepetitionPenaltyConfig,
    LiteRtLmRepetitionPenaltyConfig,
    litert_lm_repetition_penalty_config_delete
);

impl RepetitionPenaltyConfig {
    /// Wraps `litert_lm_repetition_penalty_config_create`. Defaults:
    /// `repetition_penalty` = 1.0, `presence_penalty` = 0.0,
    /// `frequency_penalty` = 0.0, `window_size` = 0 (all history, no
    /// penalties active).
    pub fn new() -> Result<Self> {
        let ptr = create("litert_lm_repetition_penalty_config_create", |out| unsafe {
            sys::litert_lm_repetition_penalty_config_create(out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_repetition_penalty_config_set_repetition_penalty`.
    pub fn set_repetition_penalty(&mut self, value: f32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_repetition_penalty_config_set_repetition_penalty(
                    self.as_ptr(),
                    value,
                )
            },
            "litert_lm_repetition_penalty_config_set_repetition_penalty",
        );
        self
    }

    /// Wraps `litert_lm_repetition_penalty_config_set_presence_penalty`.
    pub fn set_presence_penalty(&mut self, value: f32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_repetition_penalty_config_set_presence_penalty(self.as_ptr(), value)
            },
            "litert_lm_repetition_penalty_config_set_presence_penalty",
        );
        self
    }

    /// Wraps `litert_lm_repetition_penalty_config_set_frequency_penalty`.
    pub fn set_frequency_penalty(&mut self, value: f32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_repetition_penalty_config_set_frequency_penalty(self.as_ptr(), value)
            },
            "litert_lm_repetition_penalty_config_set_frequency_penalty",
        );
        self
    }

    /// Wraps `litert_lm_repetition_penalty_config_set_window_size`.
    pub fn set_window_size(&mut self, window_size: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_repetition_penalty_config_set_window_size(
                    self.as_ptr(),
                    window_size,
                )
            },
            "litert_lm_repetition_penalty_config_set_window_size",
        );
        self
    }
}

// =======================================================================
// NoRepeatNgramConfig
// =======================================================================

owned_handle!(
    /// Wraps `LiteRtLmNoRepeatNgramConfig`.
    NoRepeatNgramConfig,
    LiteRtLmNoRepeatNgramConfig,
    litert_lm_no_repeat_ngram_config_delete
);

impl NoRepeatNgramConfig {
    /// Wraps `litert_lm_no_repeat_ngram_config_create`. Defaults: size = 0,
    /// window = 0 (banning disabled).
    pub fn new() -> Result<Self> {
        let ptr = create("litert_lm_no_repeat_ngram_config_create", |out| unsafe {
            sys::litert_lm_no_repeat_ngram_config_create(out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_no_repeat_ngram_config_set_no_repeat_ngram_size`.
    pub fn set_no_repeat_ngram_size(&mut self, size: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_no_repeat_ngram_config_set_no_repeat_ngram_size(self.as_ptr(), size)
            },
            "litert_lm_no_repeat_ngram_config_set_no_repeat_ngram_size",
        );
        self
    }

    /// Wraps `litert_lm_no_repeat_ngram_config_set_window_size`.
    pub fn set_window_size(&mut self, window_size: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_no_repeat_ngram_config_set_window_size(self.as_ptr(), window_size)
            },
            "litert_lm_no_repeat_ngram_config_set_window_size",
        );
        self
    }
}

// =======================================================================
// SuppressTokensConfig
// =======================================================================

owned_handle!(
    /// Wraps `LiteRtLmSuppressTokensConfig`.
    SuppressTokensConfig,
    LiteRtLmSuppressTokensConfig,
    litert_lm_suppress_tokens_config_delete
);

impl SuppressTokensConfig {
    /// Wraps `litert_lm_suppress_tokens_config_create`. Defaults: empty set
    /// (suppression disabled).
    pub fn new() -> Result<Self> {
        let ptr = create("litert_lm_suppress_tokens_config_create", |out| unsafe {
            sys::litert_lm_suppress_tokens_config_create(out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_suppress_tokens_config_set_suppress_tokens`. Pass an
    /// empty slice to clear/disable suppression.
    pub fn set_suppress_tokens(&mut self, tokens: &[i32]) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_suppress_tokens_config_set_suppress_tokens(
                    self.as_ptr(),
                    tokens.as_ptr(),
                    tokens.len(),
                )
            },
            "litert_lm_suppress_tokens_config_set_suppress_tokens",
        );
        self
    }
}

// =======================================================================
// ThinkingConfig
// =======================================================================

owned_handle!(
    /// Wraps `LiteRtLmThinkingConfig`.
    ThinkingConfig,
    LiteRtLmThinkingConfig,
    litert_lm_thinking_config_delete
);

impl ThinkingConfig {
    /// Wraps `litert_lm_thinking_config_create`. Defaults: thinking
    /// enabled, infinite budget (-1).
    pub fn new() -> Result<Self> {
        let ptr = create("litert_lm_thinking_config_create", |out| unsafe {
            sys::litert_lm_thinking_config_create(out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_thinking_config_set_enable_thinking`.
    pub fn set_enable_thinking(&mut self, enable: bool) -> &mut Self {
        infallible(
            unsafe { sys::litert_lm_thinking_config_set_enable_thinking(self.as_ptr(), enable) },
            "litert_lm_thinking_config_set_enable_thinking",
        );
        self
    }

    /// Wraps `litert_lm_thinking_config_set_thinking_token_budget`. Pass -1
    /// for an infinite budget.
    pub fn set_thinking_token_budget(&mut self, budget: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_thinking_config_set_thinking_token_budget(self.as_ptr(), budget)
            },
            "litert_lm_thinking_config_set_thinking_token_budget",
        );
        self
    }
}

// =======================================================================
// SpeculativeDecodingConfig (new in C API 1.0.0)
// =======================================================================

owned_handle!(
    /// Wraps `LiteRtLmSpeculativeDecodingConfig`. Attach it to an
    /// [`EngineSettings`](crate::EngineSettings) and/or a [`SessionConfig`].
    SpeculativeDecodingConfig,
    LiteRtLmSpeculativeDecodingConfig,
    litert_lm_speculative_decoding_config_delete
);

impl SpeculativeDecodingConfig {
    /// Wraps `litert_lm_speculative_decoding_config_create`.
    pub fn new() -> Result<Self> {
        let ptr = create("litert_lm_speculative_decoding_config_create", |out| unsafe {
            sys::litert_lm_speculative_decoding_config_create(out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Convenience constructor that also sets the mode.
    pub fn with_mode(mode: SpeculativeDecodingMode) -> Result<Self> {
        let mut cfg = Self::new()?;
        cfg.set_mode(mode);
        Ok(cfg)
    }

    /// Wraps `litert_lm_speculative_decoding_config_set_mode`.
    pub fn set_mode(&mut self, mode: SpeculativeDecodingMode) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_speculative_decoding_config_set_mode(self.as_ptr(), mode.into())
            },
            "litert_lm_speculative_decoding_config_set_mode",
        );
        self
    }
}

// =======================================================================
// SessionConfig
// =======================================================================

owned_handle!(
    /// Wraps `LiteRtLmSessionConfig`.
    SessionConfig,
    LiteRtLmSessionConfig,
    litert_lm_session_config_delete
);

impl SessionConfig {
    /// Wraps `litert_lm_session_config_create`.
    pub fn new() -> Result<Self> {
        let ptr = create("litert_lm_session_config_create", |out| unsafe {
            sys::litert_lm_session_config_create(out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_session_config_set_max_output_tokens`. For thinking
    /// models, thinking tokens count towards this limit too.
    pub fn set_max_output_tokens(&mut self, max_output_tokens: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_session_config_set_max_output_tokens(
                    self.as_ptr(),
                    max_output_tokens,
                )
            },
            "litert_lm_session_config_set_max_output_tokens",
        );
        self
    }

    /// Wraps `litert_lm_session_config_set_apply_prompt_template`.
    pub fn set_apply_prompt_template(&mut self, apply: bool) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_session_config_set_apply_prompt_template(self.as_ptr(), apply)
            },
            "litert_lm_session_config_set_apply_prompt_template",
        );
        self
    }

    /// Wraps the deprecated `litert_lm_session_config_set_enable_speculative_decoding`.
    #[deprecated(note = "use `set_speculative_decoding_config` instead")]
    pub fn set_enable_speculative_decoding(&mut self, enable: bool) -> &mut Self {
        #[allow(deprecated)]
        let status = unsafe {
            sys::litert_lm_session_config_set_enable_speculative_decoding(self.as_ptr(), enable)
        };
        infallible(
            status,
            "litert_lm_session_config_set_enable_speculative_decoding",
        );
        self
    }

    /// Wraps `litert_lm_session_config_set_speculative_decoding_config`.
    pub fn set_speculative_decoding_config(
        &mut self,
        config: &SpeculativeDecodingConfig,
    ) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_session_config_set_speculative_decoding_config(
                    self.as_ptr(),
                    config.as_ptr(),
                )
            },
            "litert_lm_session_config_set_speculative_decoding_config",
        );
        self
    }

    /// Wraps `litert_lm_session_config_set_sampler_params`. The sampler
    /// params are copied in, so `params` can be dropped or reused
    /// afterwards.
    pub fn set_sampler_params(&mut self, params: &SamplerParams) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_session_config_set_sampler_params(self.as_ptr(), params.as_ptr())
            },
            "litert_lm_session_config_set_sampler_params",
        );
        self
    }

    /// Wraps `litert_lm_session_config_set_lora_path`.
    pub fn set_lora_path(&mut self, lora_path: &str) -> Result<&mut Self> {
        let path = cstr("lora_path", lora_path)?;
        check(
            unsafe { sys::litert_lm_session_config_set_lora_path(self.as_ptr(), path.as_ptr()) },
            "litert_lm_session_config_set_lora_path",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_session_config_set_audio_lora_path`.
    pub fn set_audio_lora_path(&mut self, audio_lora_path: &str) -> Result<&mut Self> {
        let path = cstr("audio_lora_path", audio_lora_path)?;
        check(
            unsafe {
                sys::litert_lm_session_config_set_audio_lora_path(self.as_ptr(), path.as_ptr())
            },
            "litert_lm_session_config_set_audio_lora_path",
        )?;
        Ok(self)
    }
}

// =======================================================================
// ConversationConfig
// =======================================================================

owned_handle!(
    /// Wraps `LiteRtLmConversationConfig`.
    ConversationConfig,
    LiteRtLmConversationConfig,
    litert_lm_conversation_config_delete
);

impl ConversationConfig {
    /// Wraps `litert_lm_conversation_config_create`.
    pub fn new() -> Result<Self> {
        let ptr = create("litert_lm_conversation_config_create", |out| unsafe {
            sys::litert_lm_conversation_config_create(out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps `litert_lm_conversation_config_set_session_config`.
    pub fn set_session_config(&mut self, session_config: &SessionConfig) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_config_set_session_config(
                    self.as_ptr(),
                    session_config.as_ptr(),
                )
            },
            "litert_lm_conversation_config_set_session_config",
        );
        self
    }

    /// Wraps `litert_lm_conversation_config_set_system_message`.
    pub fn set_system_message(&mut self, system_message_json: &str) -> Result<&mut Self> {
        let json = cstr("system_message_json", system_message_json)?;
        check(
            unsafe {
                sys::litert_lm_conversation_config_set_system_message(self.as_ptr(), json.as_ptr())
            },
            "litert_lm_conversation_config_set_system_message",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_tools`.
    pub fn set_tools(&mut self, tools_json: &str) -> Result<&mut Self> {
        let json = cstr("tools_json", tools_json)?;
        check(
            unsafe { sys::litert_lm_conversation_config_set_tools(self.as_ptr(), json.as_ptr()) },
            "litert_lm_conversation_config_set_tools",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_messages`.
    pub fn set_messages(&mut self, messages_json: &str) -> Result<&mut Self> {
        let json = cstr("messages_json", messages_json)?;
        check(
            unsafe {
                sys::litert_lm_conversation_config_set_messages(self.as_ptr(), json.as_ptr())
            },
            "litert_lm_conversation_config_set_messages",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_extra_context`.
    pub fn set_extra_context(&mut self, extra_context_json: &str) -> Result<&mut Self> {
        let json = cstr("extra_context_json", extra_context_json)?;
        check(
            unsafe {
                sys::litert_lm_conversation_config_set_extra_context(self.as_ptr(), json.as_ptr())
            },
            "litert_lm_conversation_config_set_extra_context",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_prompt_template`.
    pub fn set_prompt_template(&mut self, prompt_template: &str) -> Result<&mut Self> {
        let template = cstr("prompt_template", prompt_template)?;
        check(
            unsafe {
                sys::litert_lm_conversation_config_set_prompt_template(
                    self.as_ptr(),
                    template.as_ptr(),
                )
            },
            "litert_lm_conversation_config_set_prompt_template",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_enable_constrained_decoding`.
    pub fn set_enable_constrained_decoding(&mut self, enable: bool) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_config_set_enable_constrained_decoding(
                    self.as_ptr(),
                    enable,
                )
            },
            "litert_lm_conversation_config_set_enable_constrained_decoding",
        );
        self
    }

    /// Wraps `litert_lm_conversation_config_set_constraint_provider`. Pass
    /// `None` to unset (the conversation then uses its default).
    pub fn set_constraint_provider(
        &mut self,
        provider: Option<ConstraintProviderType>,
    ) -> &mut Self {
        let raw: sys::LiteRtLmConstraintProviderType = match provider {
            Some(p) => p.into(),
            None => sys::LiteRtLmConstraintProviderType_kLiteRtLmConstraintProviderTypeNone,
        };
        infallible(
            unsafe {
                sys::litert_lm_conversation_config_set_constraint_provider(self.as_ptr(), raw)
            },
            "litert_lm_conversation_config_set_constraint_provider",
        );
        self
    }

    /// Wraps
    /// `litert_lm_conversation_config_set_filter_channel_content_from_kv_cache`.
    pub fn set_filter_channel_content_from_kv_cache(&mut self, filter: bool) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_config_set_filter_channel_content_from_kv_cache(
                    self.as_ptr(),
                    filter,
                )
            },
            "litert_lm_conversation_config_set_filter_channel_content_from_kv_cache",
        );
        self
    }

    /// Wraps `litert_lm_conversation_config_set_stream_tool_calls`.
    pub fn set_stream_tool_calls(
        &mut self,
        stream: bool,
        channel_name: &str,
    ) -> Result<&mut Self> {
        let name = cstr("channel_name", channel_name)?;
        check(
            unsafe {
                sys::litert_lm_conversation_config_set_stream_tool_calls(
                    self.as_ptr(),
                    stream,
                    name.as_ptr(),
                )
            },
            "litert_lm_conversation_config_set_stream_tool_calls",
        )?;
        Ok(self)
    }

    /// Wraps `litert_lm_conversation_config_set_thinking_config`.
    pub fn set_thinking_config(&mut self, thinking_config: &ThinkingConfig) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_config_set_thinking_config(
                    self.as_ptr(),
                    thinking_config.as_ptr(),
                )
            },
            "litert_lm_conversation_config_set_thinking_config",
        );
        self
    }
}

// =======================================================================
// ConversationOptionalArgs
// =======================================================================

owned_handle!(
    /// Per-turn options for `send_message*`. Wraps
    /// `LiteRtLmConversationOptionalArgs`.
    ConversationOptionalArgs,
    LiteRtLmConversationOptionalArgs,
    litert_lm_conversation_optional_args_delete
);

impl ConversationOptionalArgs {
    /// Wraps `litert_lm_conversation_optional_args_create`.
    pub fn new() -> Result<Self> {
        let ptr = create("litert_lm_conversation_optional_args_create", |out| unsafe {
            sys::litert_lm_conversation_optional_args_create(out)
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Wraps
    /// `litert_lm_conversation_optional_args_set_repetition_penalty_config`.
    pub fn set_repetition_penalty_config(&mut self, config: &RepetitionPenaltyConfig) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_optional_args_set_repetition_penalty_config(
                    self.as_ptr(),
                    config.as_ptr(),
                )
            },
            "litert_lm_conversation_optional_args_set_repetition_penalty_config",
        );
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_no_repeat_ngram_config`.
    pub fn set_no_repeat_ngram_config(&mut self, config: &NoRepeatNgramConfig) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_optional_args_set_no_repeat_ngram_config(
                    self.as_ptr(),
                    config.as_ptr(),
                )
            },
            "litert_lm_conversation_optional_args_set_no_repeat_ngram_config",
        );
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_suppress_tokens_config`.
    pub fn set_suppress_tokens_config(&mut self, config: &SuppressTokensConfig) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_optional_args_set_suppress_tokens_config(
                    self.as_ptr(),
                    config.as_ptr(),
                )
            },
            "litert_lm_conversation_optional_args_set_suppress_tokens_config",
        );
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_visual_token_budget`.
    pub fn set_visual_token_budget(&mut self, budget: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_optional_args_set_visual_token_budget(
                    self.as_ptr(),
                    budget,
                )
            },
            "litert_lm_conversation_optional_args_set_visual_token_budget",
        );
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_max_output_tokens`.
    pub fn set_max_output_tokens(&mut self, max_output_tokens: i32) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_optional_args_set_max_output_tokens(
                    self.as_ptr(),
                    max_output_tokens,
                )
            },
            "litert_lm_conversation_optional_args_set_max_output_tokens",
        );
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_thinking_config`.
    pub fn set_thinking_config(&mut self, thinking_config: &ThinkingConfig) -> &mut Self {
        infallible(
            unsafe {
                sys::litert_lm_conversation_optional_args_set_thinking_config(
                    self.as_ptr(),
                    thinking_config.as_ptr(),
                )
            },
            "litert_lm_conversation_optional_args_set_thinking_config",
        );
        self
    }

    /// Wraps `litert_lm_conversation_optional_args_set_constraint`.
    pub fn set_constraint(
        &mut self,
        constraint_type: ConstraintType,
        constraint_string: &str,
    ) -> Result<&mut Self> {
        let constraint = cstr("constraint_string", constraint_string)?;
        check(
            unsafe {
                sys::litert_lm_conversation_optional_args_set_constraint(
                    self.as_ptr(),
                    constraint_type.into(),
                    constraint.as_ptr(),
                )
            },
            "litert_lm_conversation_optional_args_set_constraint",
        )?;
        Ok(self)
    }
}

// =======================================================================
// InputData
// =======================================================================

owned_handle!(
    /// One piece of model input (text, image or audio bytes, or an
    /// image/audio end marker). Wraps `LiteRtLmInputData`.
    InputData,
    LiteRtLmInputData,
    litert_lm_input_data_delete
);

impl InputData {
    /// Wraps `litert_lm_input_data_create`. `data` is copied internally by
    /// the library. For [`InputDataType::Text`], `data` should be UTF-8
    /// bytes; for image/audio types, the raw media bytes.
    pub fn new(data_type: InputDataType, data: &[u8]) -> Result<Self> {
        let ptr = create("litert_lm_input_data_create", |out| unsafe {
            sys::litert_lm_input_data_create(
                data_type.into(),
                data.as_ptr() as *const c_void,
                data.len(),
                out,
            )
        })?;
        Ok(Self::from_non_null(ptr))
    }

    /// Convenience constructor for [`InputDataType::Text`].
    pub fn text(text: &str) -> Result<Self> {
        Self::new(InputDataType::Text, text.as_bytes())
    }

    /// Convenience constructor for [`InputDataType::Image`] (raw image bytes).
    pub fn image(bytes: &[u8]) -> Result<Self> {
        Self::new(InputDataType::Image, bytes)
    }

    /// Convenience constructor for [`InputDataType::Audio`] (raw audio bytes).
    pub fn audio(bytes: &[u8]) -> Result<Self> {
        Self::new(InputDataType::Audio, bytes)
    }
}

/// Collects raw `const LiteRtLmInputData*` pointers for FFI calls that take
/// an array of inputs. The returned `Vec` borrows nothing, but the pointers
/// are only valid while `inputs` is alive.
pub(crate) fn input_ptrs(inputs: &[InputData]) -> Vec<*const sys::LiteRtLmInputData> {
    inputs.iter().map(|i| i.as_ptr() as *const _).collect()
}

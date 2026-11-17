//! Rust enums mirroring the C API's enums.
//!
//! Enums that are passed *into* the library convert with `From<T> for
//! LiteRtLm...`. Enums that are returned *from* the library convert with
//! `from_raw`, which returns `None` for values this crate does not know.

use litertlm_sys as sys;

/// Wraps `LiteRtLmLogSeverity`.
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

impl From<LogSeverity> for sys::LiteRtLmLogSeverity {
    fn from(level: LogSeverity) -> Self {
        match level {
            LogSeverity::Verbose => sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityVerbose,
            LogSeverity::Debug => sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityDebug,
            LogSeverity::Info => sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityInfo,
            LogSeverity::Warning => sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityWarning,
            LogSeverity::Error => sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityError,
            LogSeverity::Fatal => sys::LiteRtLmLogSeverity_kLiteRtLmLogSeverityFatal,
            LogSeverity::Silent => sys::LiteRtLmLogSeverity_kLiteRtLmLogSeveritySilent,
        }
    }
}

/// Wraps `LiteRtLmSamplerType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerType {
    /// Default/unspecified (e.g. a model that defines no default sampler).
    Unspecified,
    /// Probabilistically pick among the top-k tokens.
    TopK,
    /// Probabilistically pick among tokens whose cumulative probability
    /// reaches p, after first performing top-k sampling.
    TopP,
    /// Always pick the token with the maximum logit (argmax).
    Greedy,
}

impl From<SamplerType> for sys::LiteRtLmSamplerType {
    fn from(t: SamplerType) -> Self {
        match t {
            SamplerType::Unspecified => sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeUnspecified,
            SamplerType::TopK => sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeTopK,
            SamplerType::TopP => sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeTopP,
            SamplerType::Greedy => sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeGreedy,
        }
    }
}

impl SamplerType {
    pub(crate) fn from_raw(raw: sys::LiteRtLmSamplerType) -> Option<Self> {
        Some(match raw {
            sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeUnspecified => SamplerType::Unspecified,
            sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeTopK => SamplerType::TopK,
            sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeTopP => SamplerType::TopP,
            sys::LiteRtLmSamplerType_kLiteRtLmSamplerTypeGreedy => SamplerType::Greedy,
            _ => return None,
        })
    }
}

/// Wraps `LiteRtLmSpeculativeDecodingMode` (e.g. multi-token prediction).
/// The modes match the `--speculative_decoding` flag of the LiteRT-LM CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeculativeDecodingMode {
    /// Never speculate.
    Off,
    /// Speculate on every decode step.
    On,
    /// Speculate only while it is measured to be faster than plain decoding.
    Adaptive,
}

impl From<SpeculativeDecodingMode> for sys::LiteRtLmSpeculativeDecodingMode {
    fn from(m: SpeculativeDecodingMode) -> Self {
        match m {
            SpeculativeDecodingMode::Off => {
                sys::LiteRtLmSpeculativeDecodingMode_kLiteRtLmSpeculativeDecodingModeOff
            }
            SpeculativeDecodingMode::On => {
                sys::LiteRtLmSpeculativeDecodingMode_kLiteRtLmSpeculativeDecodingModeOn
            }
            SpeculativeDecodingMode::Adaptive => {
                sys::LiteRtLmSpeculativeDecodingMode_kLiteRtLmSpeculativeDecodingModeAdaptive
            }
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

impl From<ConstraintType> for sys::LiteRtLmConstraintType {
    fn from(t: ConstraintType) -> Self {
        match t {
            ConstraintType::None => sys::LiteRtLmConstraintType_kLiteRtLmConstraintTypeNone,
            ConstraintType::Regex => sys::LiteRtLmConstraintType_kLiteRtLmConstraintTypeRegex,
            ConstraintType::JsonSchema => {
                sys::LiteRtLmConstraintType_kLiteRtLmConstraintTypeJsonSchema
            }
        }
    }
}

/// Wraps `LiteRtLmConstraintProviderType`. "No provider" is expressed in
/// Rust as `Option::None`
/// (see [`ConversationConfig::set_constraint_provider`](crate::ConversationConfig::set_constraint_provider)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintProviderType {
    LlGuidance,
}

impl From<ConstraintProviderType> for sys::LiteRtLmConstraintProviderType {
    fn from(t: ConstraintProviderType) -> Self {
        match t {
            ConstraintProviderType::LlGuidance => {
                sys::LiteRtLmConstraintProviderType_kLiteRtLmConstraintProviderTypeLlGuidance
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

impl From<InputDataType> for sys::LiteRtLmInputDataType {
    fn from(t: InputDataType) -> Self {
        match t {
            InputDataType::Text => sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeText,
            InputDataType::Image => sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeImage,
            InputDataType::ImageEnd => sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeImageEnd,
            InputDataType::Audio => sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeAudio,
            InputDataType::AudioEnd => sys::LiteRtLmInputDataType_kLiteRtLmInputDataTypeAudioEnd,
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

impl From<ActivationDataType> for sys::LiteRtLmActivationDataType {
    fn from(t: ActivationDataType) -> Self {
        match t {
            ActivationDataType::Float32 => {
                sys::LiteRtLmActivationDataType_kLiteRtLmActivationDataTypeFloat32
            }
            ActivationDataType::Float16 => {
                sys::LiteRtLmActivationDataType_kLiteRtLmActivationDataTypeFloat16
            }
            ActivationDataType::Int16 => {
                sys::LiteRtLmActivationDataType_kLiteRtLmActivationDataTypeInt16
            }
            ActivationDataType::Int8 => {
                sys::LiteRtLmActivationDataType_kLiteRtLmActivationDataTypeInt8
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

impl TokenUnionType {
    pub(crate) fn from_raw(raw: sys::LiteRtLmTokenUnionType) -> Option<Self> {
        match raw {
            sys::LiteRtLmTokenUnionType_kLiteRtLmTokenUnionTypeString => {
                Some(TokenUnionType::String)
            }
            sys::LiteRtLmTokenUnionType_kLiteRtLmTokenUnionTypeIds => Some(TokenUnionType::Ids),
            _ => None,
        }
    }
}

/// Wraps `LiteRtLmInputOverflowStrategy`: what the embedding engine does
/// when the input is longer than the model's maximum input length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputOverflowStrategy {
    /// Split the input into chunks, embed each, and average the results.
    ChunkAndAverage,
    /// Truncate the input to the maximum length.
    Truncate,
    /// Fail with an error.
    Error,
}

impl From<InputOverflowStrategy> for sys::LiteRtLmInputOverflowStrategy {
    fn from(s: InputOverflowStrategy) -> Self {
        match s {
            InputOverflowStrategy::ChunkAndAverage => {
                sys::LiteRtLmInputOverflowStrategy_kLiteRtLmInputOverflowStrategyChunkAndAverage
            }
            InputOverflowStrategy::Truncate => {
                sys::LiteRtLmInputOverflowStrategy_kLiteRtLmInputOverflowStrategyTruncate
            }
            InputOverflowStrategy::Error => {
                sys::LiteRtLmInputOverflowStrategy_kLiteRtLmInputOverflowStrategyError
            }
        }
    }
}

impl InputOverflowStrategy {
    pub(crate) fn from_raw(raw: sys::LiteRtLmInputOverflowStrategy) -> Option<Self> {
        Some(match raw {
            sys::LiteRtLmInputOverflowStrategy_kLiteRtLmInputOverflowStrategyChunkAndAverage => {
                InputOverflowStrategy::ChunkAndAverage
            }
            sys::LiteRtLmInputOverflowStrategy_kLiteRtLmInputOverflowStrategyTruncate => {
                InputOverflowStrategy::Truncate
            }
            sys::LiteRtLmInputOverflowStrategy_kLiteRtLmInputOverflowStrategyError => {
                InputOverflowStrategy::Error
            }
            _ => return None,
        })
    }
}

/// Wraps `LiteRtLmModality`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modality {
    Text,
    Vision,
    Audio,
    Video,
}

impl From<Modality> for sys::LiteRtLmModality {
    fn from(m: Modality) -> Self {
        match m {
            Modality::Text => sys::LiteRtLmModality_kLiteRtLmModalityText,
            Modality::Vision => sys::LiteRtLmModality_kLiteRtLmModalityVision,
            Modality::Audio => sys::LiteRtLmModality_kLiteRtLmModalityAudio,
            Modality::Video => sys::LiteRtLmModality_kLiteRtLmModalityVideo,
        }
    }
}

/// Wraps `LiteRtLmBackendType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Cpu,
    Gpu,
    Npu,
}

impl BackendType {
    pub(crate) fn from_raw(raw: sys::LiteRtLmBackendType) -> Option<Self> {
        Some(match raw {
            sys::LiteRtLmBackendType_kLiteRtLmBackendTypeCpu => BackendType::Cpu,
            sys::LiteRtLmBackendType_kLiteRtLmBackendTypeGpu => BackendType::Gpu,
            sys::LiteRtLmBackendType_kLiteRtLmBackendTypeNpu => BackendType::Npu,
            _ => return None,
        })
    }
}

/// Wraps `LiteRtLmNpuBrand`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum NpuBrand {
    Unknown,
    Qualcomm,
    GoogleTensor,
    MediaTek,
    Intel,
    Samsung,
}

impl NpuBrand {
    pub(crate) fn from_raw(raw: sys::LiteRtLmNpuBrand) -> Self {
        match raw {
            sys::LiteRtLmNpuBrand_kLiteRtLmNpuBrandQualcomm => NpuBrand::Qualcomm,
            sys::LiteRtLmNpuBrand_kLiteRtLmNpuBrandGoogleTensor => NpuBrand::GoogleTensor,
            sys::LiteRtLmNpuBrand_kLiteRtLmNpuBrandMediaTek => NpuBrand::MediaTek,
            sys::LiteRtLmNpuBrand_kLiteRtLmNpuBrandIntel => NpuBrand::Intel,
            sys::LiteRtLmNpuBrand_kLiteRtLmNpuBrandSamsung => NpuBrand::Samsung,
            _ => NpuBrand::Unknown,
        }
    }
}

/// Wraps `LiteRtLmModelType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModelType {
    Unknown,
    Llm,
    Embedding,
}

impl ModelType {
    pub(crate) fn from_raw(raw: sys::LiteRtLmModelType) -> Self {
        match raw {
            sys::LiteRtLmModelType_kLiteRtLmModelTypeLlm => ModelType::Llm,
            sys::LiteRtLmModelType_kLiteRtLmModelTypeEmbedding => ModelType::Embedding,
            _ => ModelType::Unknown,
        }
    }
}

//! Experimental APIs (`experimental.h`).
//!
//! **No stability guarantees**: the C headers say these functions may change
//! or disappear in any release, so this module may too.

use litertlm_sys as sys;

use crate::conversation::Conversation;
use crate::engine::{Engine, Session};
use crate::error::Result;
use crate::util::*;

/// Wraps `litert_lm_experimental_is_debugger_enabled`: whether the library
/// was built with the tracing debugger backend (`LITERT_LM_DEBUGGER_ENABLED=1`).
pub fn is_debugger_enabled() -> Result<bool> {
    out_value("litert_lm_experimental_is_debugger_enabled", |out| unsafe {
        sys::litert_lm_experimental_is_debugger_enabled(out)
    })
}

/// Wraps `litert_lm_experimental_engine_update_gpu_enable_metal_residency_set`:
/// changes the Metal residency-set setting on an already-created engine.
pub fn update_gpu_enable_metal_residency_set(engine: &Engine, enable: bool) -> Result<()> {
    check(
        unsafe {
            sys::litert_lm_experimental_engine_update_gpu_enable_metal_residency_set(
                engine.as_ptr(),
                enable,
            )
        },
        "litert_lm_experimental_engine_update_gpu_enable_metal_residency_set",
    )
}

owned_handle!(
    /// Debug information for a session. Wraps `LiteRtLmSessionDebugInfo`.
    SessionDebugInfo,
    LiteRtLmSessionDebugInfo,
    litert_lm_experimental_session_debug_info_delete
);

impl SessionDebugInfo {
    /// Wraps `litert_lm_experimental_session_debug_info_get_capture_dir`:
    /// the (relative) directory that holds debugger captures such as
    /// safetensors dumps and token generation traces.
    pub fn capture_dir(&self) -> Result<Option<String>> {
        out_string(
            "litert_lm_experimental_session_debug_info_get_capture_dir",
            |out| unsafe {
                sys::litert_lm_experimental_session_debug_info_get_capture_dir(self.as_ptr(), out)
            },
        )
    }
}

/// Wraps `litert_lm_experimental_session_get_debug_info`.
pub fn session_debug_info(session: &Session) -> Result<SessionDebugInfo> {
    let ptr = create("litert_lm_experimental_session_get_debug_info", |out| unsafe {
        sys::litert_lm_experimental_session_get_debug_info(session.as_ptr(), out)
    })?;
    Ok(SessionDebugInfo::from_non_null(ptr))
}

/// Wraps `litert_lm_experimental_conversation_get_session_debug_info`.
pub fn conversation_session_debug_info(conversation: &Conversation) -> Result<SessionDebugInfo> {
    let ptr = create(
        "litert_lm_experimental_conversation_get_session_debug_info",
        |out| unsafe {
            sys::litert_lm_experimental_conversation_get_session_debug_info(
                conversation.as_ptr(),
                out,
            )
        },
    )?;
    Ok(SessionDebugInfo::from_non_null(ptr))
}

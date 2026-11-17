//! High-level, chat-style [`Conversation`] API.

use std::cell::Cell;
use std::ptr::NonNull;
use std::sync::Arc;

use litertlm_sys as sys;

use crate::config::ConversationOptionalArgs;
use crate::engine::{Engine, EngineHandle};
use crate::error::{Error, Result};
use crate::results::BenchmarkInfo;
use crate::stream::{run_stream, StreamChunk};
use crate::util::*;

owned_handle!(
    /// A JSON response from `send_message`. Internal; the text is copied out.
    JsonResponse,
    LiteRtLmJsonResponse,
    litert_lm_json_response_delete
);

/// Called by [`Engine`] to create a conversation (`config` may be NULL for
/// the default config).
pub(crate) fn create_conversation(
    engine: &Engine,
    config: *mut sys::LiteRtLmConversationConfig,
) -> Result<Conversation> {
    let ptr = create("litert_lm_conversation_create", |out| unsafe {
        sys::litert_lm_conversation_create(engine.as_ptr(), config, out)
    })?;
    Ok(Conversation {
        ptr,
        cancelled: Cell::new(false),
        engine: engine.handle(),
    })
}

/// A chat-style conversation with message history, tools, thinking, etc.
///
/// Holds the engine alive (see [`Engine`](crate::Engine)).
/// Not `Sync`: calls on one conversation must not overlap.
pub struct Conversation {
    ptr: NonNull<sys::LiteRtLmConversation>,
    cancelled: Cell<bool>,
    // Declared last so the engine is released after the conversation is
    // deleted.
    engine: Arc<EngineHandle>,
}

// SAFETY: see `owned_handle!`.
unsafe impl Send for Conversation {}

impl std::fmt::Debug for Conversation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conversation").finish_non_exhaustive()
    }
}

impl Conversation {
    pub(crate) fn as_ptr(&self) -> *mut sys::LiteRtLmConversation {
        self.ptr.as_ptr()
    }

    /// The C API forbids reusing a conversation after `cancel_process`.
    fn usable(&self, what: &'static str) -> Result<()> {
        if self.cancelled.get() {
            Err(Error::UseAfterCancel(what))
        } else {
            Ok(())
        }
    }

    /// Wraps `litert_lm_conversation_clone`, duplicating the prefilled
    /// state.
    pub fn clone_conversation(&self) -> Result<Conversation> {
        self.usable("litert_lm_conversation_clone")?;
        let ptr = create("litert_lm_conversation_clone", |out| unsafe {
            sys::litert_lm_conversation_clone(self.as_ptr(), out)
        })?;
        Ok(Conversation {
            ptr,
            cancelled: Cell::new(false),
            engine: self.engine.clone(),
        })
    }

    /// Sends a plain-text user message and blocks until the full response
    /// comes back. `message_json` follows the library's message JSON
    /// schema, e.g. `{"role": "user", "content": "hello"}`. Equivalent to
    /// `send_message_with_args(message_json, None, None)`.
    pub fn send_message(&self, message_json: &str) -> Result<String> {
        self.send_message_with_args(message_json, None, None)
    }

    /// Wraps `litert_lm_conversation_send_message` with optional extra
    /// context and per-turn [`ConversationOptionalArgs`].
    pub fn send_message_with_args(
        &self,
        message_json: &str,
        extra_context: Option<&str>,
        optional_args: Option<&ConversationOptionalArgs>,
    ) -> Result<String> {
        const WHAT: &str = "litert_lm_conversation_send_message";
        self.usable(WHAT)?;
        let message_c = cstr("message_json", message_json)?;
        let extra_context_c = opt_cstr("extra_context", extra_context)?;
        let args_ptr = optional_args.map_or(std::ptr::null(), |a| a.as_ptr() as *const _);

        let response = create(WHAT, |out| unsafe {
            sys::litert_lm_conversation_send_message(
                self.as_ptr(),
                message_c.as_ptr(),
                opt_ptr(&extra_context_c),
                args_ptr,
                out,
            )
        })?;
        let response = JsonResponse::from_non_null(response);
        out_string_required("litert_lm_json_response_get_string", |out| unsafe {
            sys::litert_lm_json_response_get_string(response.as_ptr(), out)
        })
    }

    /// Sends a message and streams the response, calling `on_chunk` for
    /// every chunk as it arrives. Equivalent to
    /// `send_message_stream_with_args(message_json, None, None, on_chunk)`.
    ///
    /// Blocks until the stream completes; see the
    /// [`stream`](crate::stream) module docs for why, and for the rules
    /// your callback must follow.
    pub fn send_message_stream<F: FnMut(&StreamChunk)>(
        &self,
        message_json: &str,
        on_chunk: F,
    ) -> Result<()> {
        self.send_message_stream_with_args(message_json, None, None, on_chunk)
    }

    /// Wraps `litert_lm_conversation_send_message_stream` with optional
    /// extra context and per-turn [`ConversationOptionalArgs`].
    ///
    /// If the engine was created with
    /// [`single_threaded_execution`](crate::EngineSettings::set_single_threaded_execution),
    /// this also drives the stream via `wait_until_done`, so your callback
    /// runs on the calling thread.
    pub fn send_message_stream_with_args<F: FnMut(&StreamChunk)>(
        &self,
        message_json: &str,
        extra_context: Option<&str>,
        optional_args: Option<&ConversationOptionalArgs>,
        on_chunk: F,
    ) -> Result<()> {
        const WHAT: &str = "litert_lm_conversation_send_message_stream";
        self.usable(WHAT)?;
        let message_c = cstr("message_json", message_json)?;
        let extra_context_c = opt_cstr("extra_context", extra_context)?;
        let args_ptr = optional_args.map_or(std::ptr::null(), |a| a.as_ptr() as *const _);
        let single_threaded = self.engine.single_threaded;

        run_stream(
            WHAT,
            on_chunk,
            |cb, data| unsafe {
                sys::litert_lm_conversation_send_message_stream(
                    self.as_ptr(),
                    message_c.as_ptr(),
                    opt_ptr(&extra_context_c),
                    args_ptr,
                    cb,
                    data,
                )
            },
            || {
                if single_threaded {
                    self.wait_until_done()
                } else {
                    Ok(())
                }
            },
        )
    }

    /// Wraps `litert_lm_conversation_wait_until_done`: triggers execution
    /// of, and waits for, all pending tasks in the conversation.
    ///
    /// The streaming methods call this for you when the engine is
    /// single-threaded; you normally do not need to call it yourself.
    pub fn wait_until_done(&self) -> Result<()> {
        const WHAT: &str = "litert_lm_conversation_wait_until_done";
        self.usable(WHAT)?;
        check(
            unsafe { sys::litert_lm_conversation_wait_until_done(self.as_ptr()) },
            WHAT,
        )
    }

    /// Wraps `litert_lm_conversation_render_message_to_string`. This does
    /// NOT need to be called for normal message sending -- `send_message`
    /// handles rendering internally. Copies the result into an owned
    /// `String` immediately, since the C string is only valid until the
    /// next call to this function or conversation deletion.
    pub fn render_message_to_string(&self, message_json: &str) -> Result<String> {
        const WHAT: &str = "litert_lm_conversation_render_message_to_string";
        self.usable(WHAT)?;
        let message_c = cstr("message_json", message_json)?;
        out_string_required(WHAT, |out| unsafe {
            sys::litert_lm_conversation_render_message_to_string(
                self.as_ptr(),
                message_c.as_ptr(),
                out,
            )
        })
    }

    /// Wraps `litert_lm_conversation_render_preface_to_string`.
    pub fn render_preface_to_string(&self) -> Result<String> {
        const WHAT: &str = "litert_lm_conversation_render_preface_to_string";
        self.usable(WHAT)?;
        out_string_required(WHAT, |out| unsafe {
            sys::litert_lm_conversation_render_preface_to_string(self.as_ptr(), out)
        })
    }

    /// Wraps `litert_lm_conversation_cancel_process`.
    ///
    /// Per the C API, a cancelled conversation **must only be dropped**:
    /// every further call on it returns [`Error::UseAfterCancel`]. May be
    /// called from inside a stream callback while handling a non-final
    /// chunk.
    pub fn cancel_process(&self) -> Result<()> {
        check(
            unsafe { sys::litert_lm_conversation_cancel_process(self.as_ptr()) },
            "litert_lm_conversation_cancel_process",
        )?;
        self.cancelled.set(true);
        Ok(())
    }

    /// Wraps `litert_lm_conversation_get_benchmark_info`. Requires
    /// [`EngineSettings::enable_benchmark`](crate::EngineSettings::enable_benchmark)
    /// to have been called before the engine was created.
    pub fn get_benchmark_info(&self) -> Result<BenchmarkInfo> {
        const WHAT: &str = "litert_lm_conversation_get_benchmark_info";
        self.usable(WHAT)?;
        let ptr = create(WHAT, |out| unsafe {
            sys::litert_lm_conversation_get_benchmark_info(self.as_ptr(), out)
        })?;
        Ok(BenchmarkInfo::from_non_null(ptr))
    }

    /// Wraps `litert_lm_conversation_get_token_count`: the number of
    /// tokens in the conversation's KV cache (prefill + decode).
    pub fn token_count(&self) -> Result<usize> {
        const WHAT: &str = "litert_lm_conversation_get_token_count";
        self.usable(WHAT)?;
        out_value(WHAT, |out| unsafe {
            sys::litert_lm_conversation_get_token_count(self.as_ptr(), out)
        })
    }
}

impl Drop for Conversation {
    fn drop(&mut self) {
        unsafe { sys::litert_lm_conversation_delete(self.ptr.as_ptr()) };
    }
}

//! Streaming support shared by [`Conversation`](crate::Conversation) and
//! [`Session`](crate::Session).
//!
//! The C streaming functions are asynchronous: they return immediately and
//! invoke a callback from a library-owned thread. To keep the binding safe
//! (the callback may borrow from the caller's stack) these wrappers **block
//! until the final chunk has been delivered**, so everything the closure
//! captures only has to outlive the call.
//!
//! # Rules for your callback
//!
//! From the C API's threading model:
//! * It may run on a library-owned thread shared by all streams of an
//!   engine, so keep it short.
//! * It **must not** call into LiteRT-LM on the engine that runs the
//!   stream, or on any session/conversation created from it, *except* that
//!   while handling a chunk that is **not** final it may call
//!   `cancel_process` on the session/conversation being streamed. Hand any
//!   other such work off to another thread.
//! * After `cancel_process`, that session/conversation must only be dropped.
//!
//! A panic inside the callback never unwinds across the FFI boundary: it is
//! caught, further chunks are ignored, and the panic is resumed on the
//! calling thread once the stream has finished.

use std::any::Any;
use std::ffi::c_void;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, MutexGuard};

use litertlm_sys as sys;

use crate::error::{Error, Result};
use crate::util::{check, string_from_ptr};

/// Borrowed view of a single streamed chunk. Only valid for the duration of
/// the callback that receives it -- do not store it.
pub struct StreamChunk {
    ptr: *const sys::LiteRtLmStreamChunk,
}

impl StreamChunk {
    /// Wraps `litert_lm_stream_chunk_get_text`. `None` if the chunk has no
    /// text (e.g. an error or a final metadata-only chunk).
    pub fn text(&self) -> Option<String> {
        let mut out: *const std::ffi::c_char = std::ptr::null();
        let status = unsafe { sys::litert_lm_stream_chunk_get_text(self.ptr, &mut out) };
        if status != sys::LiteRtLmStatusCode_kLiteRtLmStatusOk {
            return None;
        }
        // SAFETY: owned by the chunk, valid for the callback's duration.
        unsafe { string_from_ptr(out) }
    }

    /// Wraps `litert_lm_stream_chunk_is_final`.
    pub fn is_final(&self) -> bool {
        let mut out = false;
        let status = unsafe { sys::litert_lm_stream_chunk_is_final(self.ptr, &mut out) };
        status == sys::LiteRtLmStatusCode_kLiteRtLmStatusOk && out
    }

    /// Wraps `litert_lm_stream_chunk_get_error`. `Some(message)` if this
    /// chunk reports a stream error.
    pub fn error(&self) -> Option<String> {
        let mut out: *const std::ffi::c_char = std::ptr::null();
        let status = unsafe { sys::litert_lm_stream_chunk_get_error(self.ptr, &mut out) };
        if status != sys::LiteRtLmStatusCode_kLiteRtLmStatusOk {
            return None;
        }
        unsafe { string_from_ptr(out) }
    }
}

/// Lifetime-erased pointer to the caller's closure.
struct CallbackPtr(*mut (dyn FnMut(&StreamChunk) + 'static));
// SAFETY: only dereferenced while holding `Shared::callback`'s mutex, and
// only until `run_stream` clears it (before the closure's borrow ends).
unsafe impl Send for CallbackPtr {}

/// State shared between the calling thread and the library's callback
/// thread. Lives on the heap behind an `Arc` so a late callback can never
/// touch freed stack memory.
struct Shared {
    callback: Mutex<Option<CallbackPtr>>,
    panic: Mutex<Option<Box<dyn Any + Send>>>,
    last_error: Mutex<Option<String>>,
    done: Mutex<Sender<std::result::Result<(), String>>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

extern "C" fn trampoline(callback_data: *mut c_void, chunk: *const sys::LiteRtLmStreamChunk) {
    // SAFETY: `callback_data` came from `Arc::into_raw` in `run_stream`. We
    // borrow it here without touching the count; on the final chunk we
    // release the count that was handed to the library.
    let shared: &Shared = unsafe { &*(callback_data as *const Shared) };
    let chunk = StreamChunk { ptr: chunk };
    let is_final = chunk.is_final();
    if let Some(e) = chunk.error() {
        *lock(&shared.last_error) = Some(e);
    }

    // Run the user's closure, shielding the FFI boundary from panics.
    {
        let guard = lock(&shared.callback);
        if let Some(CallbackPtr(cb)) = guard.as_ref() {
            if lock(&shared.panic).is_none() {
                let cb = *cb;
                // SAFETY: valid until `run_stream` clears it under this same
                // mutex, which cannot happen while we hold the guard.
                let result = catch_unwind(AssertUnwindSafe(|| unsafe { (*cb)(&chunk) }));
                if let Err(p) = result {
                    *lock(&shared.panic) = Some(p);
                }
            }
        }
    }

    if is_final {
        let outcome = match lock(&shared.last_error).take() {
            Some(msg) => Err(msg),
            None => Ok(()),
        };
        let tx = lock(&shared.done).clone();
        let _ = tx.send(outcome);
        // Release the strong count that was given to the library. The
        // calling thread holds its own, so `shared` stays alive for it.
        // SAFETY: balances the `Arc::into_raw` in `run_stream`; the library
        // delivers no chunks after the final one.
        drop(unsafe { Arc::from_raw(callback_data as *const Shared) });
    }
}

/// Starts a stream with `start`, optionally drives it (single-threaded
/// engines), and blocks until the final chunk arrives.
///
/// `start` performs the actual FFI call, given the trampoline and its opaque
/// data pointer. `drive` runs after a successful start and may block while
/// the library delivers chunks (e.g. `litert_lm_conversation_wait_until_done`).
pub(crate) fn run_stream<F: FnMut(&StreamChunk)>(
    what: &'static str,
    mut on_chunk: F,
    start: impl FnOnce(sys::LiteRtLmStreamCallback, *mut c_void) -> sys::LiteRtLmStatusCode,
    drive: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let (tx, rx) = channel();

    // Erase the closure's lifetime. Soundness: we clear the pointer (under
    // the mutex the callback holds while running) before returning, on every
    // path, so it is never dereferenced after `on_chunk` goes out of scope.
    let erased: *mut (dyn FnMut(&StreamChunk) + '_) = &mut on_chunk;
    let erased: *mut (dyn FnMut(&StreamChunk) + 'static) = unsafe { std::mem::transmute(erased) };

    let shared = Arc::new(Shared {
        callback: Mutex::new(Some(CallbackPtr(erased))),
        panic: Mutex::new(None),
        last_error: Mutex::new(None),
        done: Mutex::new(tx),
    });

    // One strong count is lent to the library for the callback data.
    let data = Arc::into_raw(shared.clone()) as *mut c_void;

    let status = start(Some(trampoline), data);
    if let Err(e) = check(status, what) {
        // The stream never started; take back the count we lent out.
        *lock(&shared.callback) = None;
        drop(unsafe { Arc::from_raw(data as *const Shared) });
        return Err(e);
    }

    if let Err(e) = drive() {
        // We cannot know whether the library will still call back. Detach
        // the closure and deliberately leak the lent count so a late
        // callback only ever sees valid heap memory.
        *lock(&shared.callback) = None;
        return Err(e);
    }

    let outcome = rx.recv();

    // Detach the closure before it goes out of scope.
    *lock(&shared.callback) = None;

    if let Some(p) = lock(&shared.panic).take() {
        resume_unwind(p);
    }

    match outcome {
        Ok(Ok(())) => Ok(()),
        Ok(Err(msg)) => Err(Error::Stream(msg)),
        Err(_) => Err(Error::Stream(
            "stream channel closed before a final chunk arrived".into(),
        )),
    }
}

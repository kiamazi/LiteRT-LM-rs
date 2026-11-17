//! Error type for this crate.
//!
//! The LiteRT-LM C API (1.0.0+) reports every failure as a
//! `LiteRtLmStatusCode` and records a human-readable message in thread-local
//! storage. This module turns both into a single [`Error`] value. The
//! message is copied out **immediately** on the thread that observed the
//! failure (as the C API requires), so an [`Error`] can be freely moved to
//! other threads.

use std::ffi::{CStr, NulError};
use std::fmt;

use litertlm_sys as sys;

use crate::version::ApiVersion;

/// Convenience alias for `Result<T, litertlm_rs::Error>`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Mirror of `LiteRtLmStatusCode` (the canonical Google/absl status codes),
/// minus `Ok`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusCode {
    Cancelled,
    Unknown,
    InvalidArgument,
    DeadlineExceeded,
    NotFound,
    AlreadyExists,
    PermissionDenied,
    ResourceExhausted,
    FailedPrecondition,
    Aborted,
    OutOfRange,
    Unimplemented,
    Internal,
    Unavailable,
    DataLoss,
    Unauthenticated,
}

impl StatusCode {
    /// Converts a raw non-zero `LiteRtLmStatusCode`. Unrecognized values map
    /// to [`StatusCode::Unknown`], as the C API header recommends.
    pub fn from_raw(raw: sys::LiteRtLmStatusCode) -> Self {
        use StatusCode::*;
        match raw {
            sys::LiteRtLmStatusCode_kLiteRtLmStatusCancelled => Cancelled,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusInvalidArgument => InvalidArgument,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusDeadlineExceeded => DeadlineExceeded,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusNotFound => NotFound,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusAlreadyExists => AlreadyExists,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusPermissionDenied => PermissionDenied,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusResourceExhausted => ResourceExhausted,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusFailedPrecondition => FailedPrecondition,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusAborted => Aborted,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusOutOfRange => OutOfRange,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusUnimplemented => Unimplemented,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusInternal => Internal,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusUnavailable => Unavailable,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusDataLoss => DataLoss,
            sys::LiteRtLmStatusCode_kLiteRtLmStatusUnauthenticated => Unauthenticated,
            _ => Unknown,
        }
    }

    /// The raw numeric value of this code, as defined by the C API.
    pub fn as_raw(self) -> sys::LiteRtLmStatusCode {
        use StatusCode::*;
        match self {
            Cancelled => sys::LiteRtLmStatusCode_kLiteRtLmStatusCancelled,
            Unknown => sys::LiteRtLmStatusCode_kLiteRtLmStatusUnknown,
            InvalidArgument => sys::LiteRtLmStatusCode_kLiteRtLmStatusInvalidArgument,
            DeadlineExceeded => sys::LiteRtLmStatusCode_kLiteRtLmStatusDeadlineExceeded,
            NotFound => sys::LiteRtLmStatusCode_kLiteRtLmStatusNotFound,
            AlreadyExists => sys::LiteRtLmStatusCode_kLiteRtLmStatusAlreadyExists,
            PermissionDenied => sys::LiteRtLmStatusCode_kLiteRtLmStatusPermissionDenied,
            ResourceExhausted => sys::LiteRtLmStatusCode_kLiteRtLmStatusResourceExhausted,
            FailedPrecondition => sys::LiteRtLmStatusCode_kLiteRtLmStatusFailedPrecondition,
            Aborted => sys::LiteRtLmStatusCode_kLiteRtLmStatusAborted,
            OutOfRange => sys::LiteRtLmStatusCode_kLiteRtLmStatusOutOfRange,
            Unimplemented => sys::LiteRtLmStatusCode_kLiteRtLmStatusUnimplemented,
            Internal => sys::LiteRtLmStatusCode_kLiteRtLmStatusInternal,
            Unavailable => sys::LiteRtLmStatusCode_kLiteRtLmStatusUnavailable,
            DataLoss => sys::LiteRtLmStatusCode_kLiteRtLmStatusDataLoss,
            Unauthenticated => sys::LiteRtLmStatusCode_kLiteRtLmStatusUnauthenticated,
        }
    }
}

impl fmt::Display for StatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StatusCode::Cancelled => "cancelled",
            StatusCode::Unknown => "unknown",
            StatusCode::InvalidArgument => "invalid argument",
            StatusCode::DeadlineExceeded => "deadline exceeded",
            StatusCode::NotFound => "not found",
            StatusCode::AlreadyExists => "already exists",
            StatusCode::PermissionDenied => "permission denied",
            StatusCode::ResourceExhausted => "resource exhausted",
            StatusCode::FailedPrecondition => "failed precondition",
            StatusCode::Aborted => "aborted",
            StatusCode::OutOfRange => "out of range",
            StatusCode::Unimplemented => "unimplemented",
            StatusCode::Internal => "internal",
            StatusCode::Unavailable => "unavailable",
            StatusCode::DataLoss => "data loss",
            StatusCode::Unauthenticated => "unauthenticated",
        };
        f.write_str(s)
    }
}

/// Errors returned by this crate.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A Rust string passed to the C API contained an interior NUL byte.
    NulArgument(&'static str),
    /// A C API call returned a non-OK status code.
    Status {
        /// The `litert_lm_*` function that failed.
        what: &'static str,
        /// The status code it returned.
        code: StatusCode,
        /// The library's last-error message for the failing thread, if any.
        message: Option<String>,
    },
    /// A C API call succeeded but produced no value where one was required
    /// (the C API reports "legitimately absent" values as success + NULL).
    Null(&'static str),
    /// The C API returned an enum value this crate does not know about.
    UnexpectedValue {
        what: &'static str,
        value: i64,
    },
    /// The stream ended with an error chunk.
    Stream(String),
    /// The session or conversation was cancelled; the C API requires that
    /// it is only deleted afterwards, never reused.
    UseAfterCancel(&'static str),
    /// The operation is not available in the current configuration.
    Unsupported(&'static str),
    /// The loaded `liblitert-lm` is not ABI-compatible with the headers
    /// this crate was built against.
    IncompatibleLibrary {
        /// Version of the C API headers this crate was compiled against.
        headers: ApiVersion,
        /// Version reported by the loaded library.
        library: ApiVersion,
    },
}

impl Error {
    /// Builds an [`Error::Status`], capturing the calling thread's last
    /// error message immediately (it is thread-local and short-lived).
    pub(crate) fn from_status(raw: sys::LiteRtLmStatusCode, what: &'static str) -> Self {
        Error::Status {
            what,
            code: StatusCode::from_raw(raw),
            message: last_error_message(),
        }
    }

    /// The C API status code, if this error came from one.
    pub fn code(&self) -> Option<StatusCode> {
        match self {
            Error::Status { code, .. } => Some(*code),
            _ => None,
        }
    }

    /// `true` for `kLiteRtLmStatusNotFound`, which many query functions use
    /// to say "this model does not define that property".
    pub fn is_not_found(&self) -> bool {
        self.code() == Some(StatusCode::NotFound)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NulArgument(name) => write!(f, "argument `{name}` contained a NUL byte"),
            Error::Status {
                what,
                code,
                message,
            } => match message {
                Some(m) if !m.is_empty() => write!(f, "{what} failed ({code}): {m}"),
                _ => write!(f, "{what} failed ({code})"),
            },
            Error::Null(what) => write!(f, "{what} returned no value"),
            Error::UnexpectedValue { what, value } => {
                write!(f, "{what} returned an unrecognized value ({value})")
            }
            Error::Stream(msg) => write!(f, "stream error: {msg}"),
            Error::UseAfterCancel(what) => write!(
                f,
                "{what}: this handle was cancelled and must only be dropped, not reused"
            ),
            Error::Unsupported(what) => write!(f, "unsupported: {what}"),
            Error::IncompatibleLibrary { headers, library } => write!(
                f,
                "liblitert-lm reports C API {library}, which is not compatible with the \
                 {headers} headers this crate was built against (major versions must match \
                 and the library's minor version must be >= the headers')"
            ),
        }
    }
}

impl std::error::Error for Error {}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Error::NulArgument("<string argument>")
    }
}

/// Returns the last error message recorded by the C library **on the
/// calling thread**, if any.
///
/// You normally never need this: every [`Error::Status`] already carries the
/// message captured at the moment of failure. It is exposed for completeness
/// (`litert_lm_get_last_error_message`).
pub fn last_error_message() -> Option<String> {
    // SAFETY: the returned pointer is owned by the library's thread-local
    // storage and valid until the next failing call on this thread; we copy
    // it out before doing anything else.
    unsafe {
        let ptr = sys::litert_lm_get_last_error_message();
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
    }
}

/// Clears the calling thread's last-error state
/// (`litert_lm_clear_last_error`). Optional; see `error_reporter.h`.
pub fn clear_last_error() {
    unsafe { sys::litert_lm_clear_last_error() };
}

//! Crate-private FFI helpers shared by every module.

use std::ffi::{c_char, CStr, CString};
use std::ptr::NonNull;

use litertlm_sys as sys;

use crate::error::{Error, Result};

/// Converts a Rust string to a `CString`, naming the argument in the error.
pub(crate) fn cstr(arg: &'static str, s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| Error::NulArgument(arg))
}

/// Like [`cstr`] for optional arguments.
pub(crate) fn opt_cstr(arg: &'static str, s: Option<&str>) -> Result<Option<CString>> {
    s.map(|s| cstr(arg, s)).transpose()
}

/// Raw pointer for an optional `CString` (NULL when absent).
pub(crate) fn opt_ptr(s: &Option<CString>) -> *const c_char {
    s.as_ref().map_or(std::ptr::null(), |s| s.as_ptr())
}

/// Copies a library-owned C string into an owned `String` (lossy UTF-8).
///
/// # Safety
/// `ptr` must be NULL or point at a valid NUL-terminated string.
pub(crate) unsafe fn string_from_ptr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

/// `Ok(())` for `kLiteRtLmStatusOk`, otherwise an [`Error::Status`] carrying
/// the thread-local message. Must be called on the thread that made the FFI
/// call, before any other LiteRT-LM call.
pub(crate) fn check(status: sys::LiteRtLmStatusCode, what: &'static str) -> Result<()> {
    if status == sys::LiteRtLmStatusCode_kLiteRtLmStatusOk {
        Ok(())
    } else {
        Err(Error::from_status(status, what))
    }
}

/// For setters whose only documented failure is a NULL handle/argument,
/// which the Rust types make impossible. A non-OK status therefore means the
/// library broke its contract, which we surface loudly instead of silently
/// ignoring.
#[track_caller]
pub(crate) fn infallible(status: sys::LiteRtLmStatusCode, what: &'static str) {
    if let Err(e) = check(status, what) {
        panic!("{e} (this call can only fail on a NULL argument, which is impossible here)");
    }
}

/// Calls a C `*_create`-style function with a `T**` out-parameter and
/// returns the owned, non-NULL handle.
pub(crate) fn create<T>(
    what: &'static str,
    f: impl FnOnce(*mut *mut T) -> sys::LiteRtLmStatusCode,
) -> Result<NonNull<T>> {
    let mut out: *mut T = std::ptr::null_mut();
    check(f(&mut out), what)?;
    NonNull::new(out).ok_or(Error::Null(what))
}

/// Like [`create`], but a NULL result on success is `Ok(None)` (used for
/// values that can be legitimately absent, e.g. a missing start token).
pub(crate) fn create_optional<T>(
    what: &'static str,
    f: impl FnOnce(*mut *mut T) -> sys::LiteRtLmStatusCode,
) -> Result<Option<NonNull<T>>> {
    let mut out: *mut T = std::ptr::null_mut();
    check(f(&mut out), what)?;
    Ok(NonNull::new(out))
}

/// Calls a C getter with a scalar out-parameter.
pub(crate) fn out_value<T: Default>(
    what: &'static str,
    f: impl FnOnce(*mut T) -> sys::LiteRtLmStatusCode,
) -> Result<T> {
    let mut out = T::default();
    check(f(&mut out), what)?;
    Ok(out)
}

/// Calls a C getter with a `const char**` out-parameter and copies the
/// string. `Ok(None)` when the library reports "no value" (success + NULL).
pub(crate) fn out_string(
    what: &'static str,
    f: impl FnOnce(*mut *const c_char) -> sys::LiteRtLmStatusCode,
) -> Result<Option<String>> {
    let mut out: *const c_char = std::ptr::null();
    check(f(&mut out), what)?;
    // SAFETY: on success the library hands back NULL or a valid C string
    // that outlives this call (it is owned by the object we were called on).
    Ok(unsafe { string_from_ptr(out) })
}

/// Like [`out_string`] but NULL is an error.
pub(crate) fn out_string_required(
    what: &'static str,
    f: impl FnOnce(*mut *const c_char) -> sys::LiteRtLmStatusCode,
) -> Result<String> {
    out_string(what, f)?.ok_or(Error::Null(what))
}

/// Defines an owned FFI handle type: a non-NULL pointer that is `Send`, not
/// `Sync` (the C API forbids overlapping calls on one handle), and released
/// with `$delete` on drop.
macro_rules! owned_handle {
    ($(#[$meta:meta])* $name:ident, $sys_ty:ident, $delete:ident) => {
        $(#[$meta])*
        pub struct $name {
            ptr: ::std::ptr::NonNull<::litertlm_sys::$sys_ty>,
        }

        // SAFETY: the C API requires only that calls using the *same handle*
        // do not overlap; moving a handle between threads is fine. The type
        // is intentionally not `Sync`.
        unsafe impl Send for $name {}

        #[allow(dead_code)]
        impl $name {
            pub(crate) fn from_non_null(
                ptr: ::std::ptr::NonNull<::litertlm_sys::$sys_ty>,
            ) -> Self {
                Self { ptr }
            }

            pub(crate) fn as_ptr(&self) -> *mut ::litertlm_sys::$sys_ty {
                self.ptr.as_ptr()
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                // SAFETY: `ptr` is a live handle owned by `self`.
                unsafe { ::litertlm_sys::$delete(self.ptr.as_ptr()) };
            }
        }

        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(stringify!($name)).finish_non_exhaustive()
            }
        }
    };
}


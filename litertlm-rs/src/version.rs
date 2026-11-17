//! C API version handling (`litert_lm_get_api_version`).
//!
//! The LiteRT-LM C API is versioned independently of LiteRT-LM releases
//! using semantic versioning: a different **major** version is
//! ABI-incompatible, and an older **minor** version may lack functions the
//! headers declare. This crate verifies that once, lazily, before it creates
//! any engine, so a mismatched `liblitert-lm` fails loudly with
//! [`Error::IncompatibleLibrary`] instead of corrupting memory.

use std::fmt;
use std::sync::OnceLock;

use litertlm_sys as sys;

use crate::error::{Error, Result};
use crate::util::{check, infallible};

/// A C API version (`MAJOR.MINOR.PATCH`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiVersion {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
}

impl ApiVersion {
    /// The C API version of the headers this crate was compiled against
    /// (`LITERT_LM_C_API_VERSION_*`).
    pub const HEADERS: ApiVersion = ApiVersion {
        major: sys::LITERT_LM_C_API_VERSION_MAJOR as i32,
        minor: sys::LITERT_LM_C_API_VERSION_MINOR as i32,
        patch: sys::LITERT_LM_C_API_VERSION_PATCH as i32,
    };

    /// Whether a library reporting `library` can be used by code compiled
    /// against `self` (same major, library minor not older).
    pub fn is_compatible_with_library(&self, library: ApiVersion) -> bool {
        library.major == self.major && library.minor >= self.minor
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Queries the C API version of the loaded library
/// (`litert_lm_get_api_version`).
pub fn api_version() -> Result<ApiVersion> {
    let (mut major, mut minor, mut patch) = (0, 0, 0);
    check(
        unsafe { sys::litert_lm_get_api_version(&mut major, &mut minor, &mut patch) },
        "litert_lm_get_api_version",
    )?;
    Ok(ApiVersion {
        major,
        minor,
        patch,
    })
}

/// Verifies that the loaded library is compatible with the headers this
/// crate was built against. The (cheap) library query is cached after the
/// first success.
pub fn check_api_version() -> Result<()> {
    static LIBRARY: OnceLock<ApiVersion> = OnceLock::new();
    let library = match LIBRARY.get() {
        Some(v) => *v,
        None => {
            let v = api_version()?;
            *LIBRARY.get_or_init(|| v)
        }
    };
    if ApiVersion::HEADERS.is_compatible_with_library(library) {
        Ok(())
    } else {
        Err(Error::IncompatibleLibrary {
            headers: ApiVersion::HEADERS,
            library,
        })
    }
}

// Silences "unused" if a future change stops using it in some cfg.
#[allow(dead_code)]
fn _assert_helpers_linked() {
    let _ = infallible;
}

//! `litertlm-sys` finds/downloads/copies `liblitert-lm.*` into its own
//! `OUT_DIR` and makes cargo link against it there -- that's enough for
//! *compiling*. But `rustc-link-arg` (used for rpath) only applies to
//! binaries built in the package that emits it, and litertlm-sys has none.
//! So it's this build script's job to actually get the library next to the
//! binaries that get produced (this crate's examples/tests, and any
//! downstream consumer's binary) and to point them at it via rpath.
//!
//! `litertlm-sys` hands us the library's location through Cargo's
//! `links = "litert-lm"` build-script metadata mechanism, exposed here as
//! `DEP_LITERT_LM_LIB_DIR` / `DEP_LITERT_LM_LIB_FILENAME`.

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    if env::var("DOCS_RS").is_ok() {
        return;
    }

    let lib_dir = PathBuf::from(
        env::var("DEP_LITERT_LM_LIB_DIR")
            .expect("litertlm-sys did not report DEP_LITERT_LM_LIB_DIR -- is it a dependency?"),
    );
    let lib_filename = env::var("DEP_LITERT_LM_LIB_FILENAME")
        .expect("litertlm-sys did not report DEP_LITERT_LM_LIB_FILENAME");
    let lib_src = lib_dir.join(&lib_filename);

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // OUT_DIR is always `<target-dir>/<profile>/build/<pkg>-<hash>/out` (or
    // `<target-dir>/<triple>/<profile>/build/...` when cross-compiling), so
    // three levels up is `<target-dir>/<profile>` (or `.../<triple>/<profile>`)
    // -- the directory cargo actually places binaries, examples, and test
    // executables under.
    let profile_dir = out_dir
        .parent() // .../build/<pkg>-<hash>
        .and_then(Path::parent) // .../build
        .and_then(Path::parent) // .../<profile>
        .unwrap_or(&out_dir)
        .to_path_buf();

    // Best-effort copy the library flat next to every kind of binary this
    // build might produce, so it's found at `$ORIGIN` without needing a
    // `lib/` subfolder (whose relative depth would differ between a plain
    // binary, `examples/`, and `deps/`).
    for dest_dir in [
        profile_dir.clone(),
        profile_dir.join("examples"),
        profile_dir.join("deps"),
    ] {
        if std::fs::create_dir_all(&dest_dir).is_ok() {
            if let Err(e) = std::fs::copy(&lib_src, dest_dir.join(&lib_filename)) {
                println!(
                    "cargo:warning=Could not copy {:?} into {:?}: {}",
                    lib_src, dest_dir, e
                );
            }
        }
    }

    // rpath is an ELF/Mach-O concept; Windows resolves DLLs by searching next
    // to the .exe, which the copy step above already handles. Note the
    // "next to the binary" token differs: ELF (Linux/Android) uses
    // `$ORIGIN`, Mach-O (macOS) uses `@loader_path` -- `$ORIGIN` is silently
    // ignored by Apple's linker.
    let origin_token = match target_os.as_str() {
        "macos" => Some("@loader_path"),
        "windows" => None,
        _ => Some("$ORIGIN"),
    };
    if let Some(origin) = origin_token {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{origin}");
        // Fallback for binaries built outside the locations copied above
        // (e.g. a custom --target-dir layout): point straight at the
        // build-time location too.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }

    println!("cargo:rerun-if-changed=build.rs");
}

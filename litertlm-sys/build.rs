use std::env;
use std::path::PathBuf;
use std::path::Path;
use std::fs;
use sha2::{Sha256, Digest};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    if env::var("DOCS_RS").is_ok() {
        generate_bindings(&manifest_dir);
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();
    let profile = env::var("PROFILE").unwrap();

    println!("cargo:warning=Building litertlm-sys for target: {}", target);
    println!("cargo:warning=Profile: {}", profile);

    // Determine the library file name, subdirectory, download URL, checksum, and (for
    // Windows only) the extra `.lib` import library MSVC's linker needs
    // alongside the `.dll` -- the DLL alone is enough at runtime, but the
    // linker needs the import lib present at compile time to resolve
    // `litert_lm_*` symbols against.
    let (lib_filename, subdir, url, expected_checksum, extra_lib): (&str, &str, &str, &str, Option<(&str, &str, &str)>) =
        match target.as_str() {
            "aarch64-unknown-linux-gnu" | "aarch64-linux-gnu" => (
                "liblitert-lm.so",
                "lib/linux_arm64",
                "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/linux_arm64_liblitert-lm.so",
                "1c48e5d711069544156cd8aad61cec561262248f344cbe4ba72d8640efb4f34c",
                None,
            ),
            "x86_64-linux-gnu" | "x86_64-unknown-linux-gnu" => (
                "liblitert-lm.so",
                "lib/linux_x86_64",
                "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/linux_x86_64_liblitert-lm.so",
                "34eb4998b1b3a8b049a6fb24c8069933821abb22680f4737b7f29baae233ae35",
                None,
            ),
            "aarch64-linux-android" => (
                "liblitert-lm.so",
                "lib/android_arm64",
                "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/android_arm64_liblitert-lm.so",
                "e9cbdddb0f1c693c549e1cde40bf90ad8aaa124d15944d0dd18faaf016dd6938",
                None,
            ),
            "x86_64-linux-android" => (
                "liblitert-lm.so",
                "lib/android_x86_64",
                "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/android_x86_64_liblitert-lm.so",
                "b2976e7d57040617c541c04104cde0fa40f10bea9787fe0d21f949a97a926dff",
                None,
            ),
            "aarch64-apple-darwin" => (
                "liblitert-lm.dylib",
                "lib/macos_arm64",
                "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/liblitert-lm.dylib",
                "626a0b40a28405529d2e43ab058b4deac967cb8517cd0c37c103baa3517bee13",
                None,
            ),
            "x86_64-apple-darwin" => panic!(
                "litert-lm has no macOS x86_64 (Intel) prebuilt upstream -- only \
                 aarch64-apple-darwin is published. You'll need to build LiteRT-LM \
                 from source for Intel Macs: https://github.com/google-ai-edge/LiteRT-LM"
            ),
            "x86_64-pc-windows-msvc" => (
                "litert-lm.dll",
                "lib/windows_x86_64/bin",
                "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/windows_x86_64_litert-lm.dll",
                "4bb6bc031c5ee1588b504cb6c588b7369a99c1f4874745549abb6d4a9cb153c4",
                Some((
                    "litert-lm.lib",
                    "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/windows_x86_64_litert-lm.lib",
                    "e630a50dc9923fa03f103a34ebd7eab13ac0479b29c1b8a99bc2f9afac089103",
                )),
            ),
            "x86_64-pc-windows-gnu" => panic!(
                "litert-lm's Windows prebuilt only ships an MSVC-format import \
                 library (.lib), which GNU ld / MinGW can't link against directly. \
                 Use the MSVC target instead: `rustup target add x86_64-pc-windows-msvc`. \
                 Cross-compiling from Linux/macOS? Use cargo-xwin: \
                 `cargo install cargo-xwin && cargo xwin build --target x86_64-pc-windows-msvc`"
            ),
            _ => panic!("Unsupported target: {}", target),
        };

    // Create subdirectory structure in OUT_DIR
    let lib_subdir = out_dir.join(subdir);
    std::fs::create_dir_all(&lib_subdir)
        .unwrap_or_else(|e| panic!("Failed to create directory {:?}: {}", lib_subdir, e));

    let lib_path = lib_subdir.join(lib_filename);
    let extra_lib_path = extra_lib.map(|(filename, _, _)| lib_subdir.join(filename));

    // Priority 1: Check LITERT_LM_LIB_DIR environment variable
    if let Ok(env_lib_dir) = env::var("LITERT_LM_LIB_DIR") {
        let env_lib_path = PathBuf::from(&env_lib_dir)
            .join(subdir)
            .join(lib_filename);
        if env_lib_path.exists() {
            println!(
                "cargo:warning=Found library in LITERT_LM_LIB_DIR: {:?}",
                env_lib_path
            );
            if lib_path.exists() {
                let _ = std::fs::remove_file(&lib_path);
            }
            std::fs::copy(&env_lib_path, &lib_path).unwrap_or_else(|e| {
                panic!(
                    "Failed to copy library from {:?} to {:?}: {}",
                    env_lib_path, lib_path, e
                )
            });
            if let (Some((extra_filename, _, _)), Some(extra_dest)) = (extra_lib, &extra_lib_path) {
                let env_extra_path = PathBuf::from(&env_lib_dir).join(subdir).join(extra_filename);
                std::fs::copy(&env_extra_path, extra_dest).unwrap_or_else(|e| {
                    panic!(
                        "Failed to copy {:?} to {:?}: {} (expected alongside {} in LITERT_LM_LIB_DIR)",
                        env_extra_path, extra_dest, e, lib_filename
                    )
                });
            }
            configure_linking(&lib_subdir, lib_filename);
            generate_bindings(&manifest_dir);
            return;
        } else {
            println!(
                "cargo:warning=LITERT_LM_LIB_DIR set but library not found at: {:?}",
                env_lib_path
            );
        }
    }

    // Priority 2: Check the manifest directory itself (for crates with local prebuilt/)
    let prebuilt_lib_path = manifest_dir
        .join("prebuilt")
        .join(subdir)
        .join(lib_filename);

    if prebuilt_lib_path.exists() {
        println!(
            "cargo:warning=Found library in manifest directory: {:?}",
            prebuilt_lib_path
        );
        std::fs::copy(&prebuilt_lib_path, &lib_path).unwrap_or_else(|e| {
            panic!("Failed to copy library: {}", e)
        });
        if let (Some((extra_filename, _, _)), Some(extra_dest)) = (extra_lib, &extra_lib_path) {
            let prebuilt_extra_path = manifest_dir.join("prebuilt").join(subdir).join(extra_filename);
            std::fs::copy(&prebuilt_extra_path, extra_dest).unwrap_or_else(|e| {
                panic!(
                    "Failed to copy {:?}: {} (expected alongside {} in prebuilt/{})",
                    prebuilt_extra_path, e, lib_filename, subdir
                )
            });
        }
        configure_linking(&lib_subdir, lib_filename);
        generate_bindings(&manifest_dir);
        return;
    }

    // Priority 3: Download from GitHub as fallback
    println!(
        "cargo:warning=No prebuilt library found, downloading from GitHub"
    );
    download_file(url, &lib_path);
    verify_checksum(&lib_path, expected_checksum)
        .unwrap_or_else(|e| panic!("Checksum verification failed for library: {}", e));

    if let Some((_, extra_url, extra_checksum)) = extra_lib {
        if let Some(extra_dest) = &extra_lib_path {
            download_file(extra_url, extra_dest);
            verify_checksum(extra_dest, extra_checksum)
                .unwrap_or_else(|e| panic!("Checksum verification failed for extra library: {}", e));
        }
    }

    configure_linking(&lib_subdir, lib_filename);
    generate_bindings(&manifest_dir);
}

fn configure_linking(lib_subdir: &PathBuf, lib_filename: &str) {
    // Tell cargo where to find the library
    println!("cargo:rustc-link-search=native={}", lib_subdir.display());
    println!("cargo:rustc-link-lib=dylib=litert-lm");

    // `rustc-link-arg` only affects binaries built in *this* package
    // (litertlm-sys has none), so it does nothing useful here on its own.
    // What actually matters for downstream binaries is the `links = "litert-lm"`
    // metadata below: it exposes this directory to `litertlm-rs`'s build
    // script (as `DEP_LITERT_LM_LIB_DIR` / `DEP_LITERT_LM_LIB_FILENAME`),
    // which is what actually copies the library next to the final binary
    // and embeds the rpath pointing at it. See litertlm-rs/build.rs.
    println!("cargo:lib_dir={}", lib_subdir.display());
    println!("cargo:lib_filename={}", lib_filename);

    // Tell cargo to invalidate the built crate whenever build.rs changes
    println!("cargo:rerun-if-changed=build.rs");
}

fn generate_bindings(manifest_dir: &PathBuf) {
    let include_dir = manifest_dir.join("prebuilt/include");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", include_dir.display()))
        .derive_default(true)
        .derive_debug(true)
        .allowlist_function("litert_lm_.*")
        .allowlist_type("LiteRtLm.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings.rs");
}

fn download_file(url: &str, destination: &PathBuf) {
    use std::fs::File;
    use std::io::Write;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 minutes
        .build()
        .unwrap();

    let response = client
        .get(url)
        .send()
        .unwrap_or_else(|e| panic!("Failed to download from {}: {}", url, e));

    let bytes = response
        .bytes()
        .unwrap_or_else(|e| panic!("Failed to read response body: {}", e));

    let mut file = File::create(destination)
        .unwrap_or_else(|e| panic!("Failed to create file {:?}: {}", destination, e));

    file.write_all(&bytes)
        .unwrap_or_else(|e| panic!("Failed to write to file {:?}: {}", destination, e));

    println!(
        "cargo:warning=Downloaded {} ({} bytes)",
        url,
        bytes.len()
    );
}

/// Compute the SHA-256 hash of a file.
fn compute_sha256(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let hash_bytes = hasher.finalize();
    Ok(format!("{:x}", hash_bytes))
}

/// Verify that a file's SHA-256 hash matches the expected value.
fn verify_checksum(path: &Path, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let actual = compute_sha256(path)?;
    if actual != expected {
        return Err(format!(
            "Checksum mismatch for {}\n  Expected: {}\n  Actual:   {}",
            path.display(),
            expected,
            actual
        ).into());
    }
    println!("cargo:warning=Checksum verified for {}", path.display());
    Ok(())
}

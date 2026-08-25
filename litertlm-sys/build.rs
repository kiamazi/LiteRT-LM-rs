use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = env::var("TARGET").unwrap();
    // let profile = env::var("PROFILE").unwrap();

    // Determine the library file name, subdirectory, and download URL based on target
    let (lib_filename, subdir, url) = match target.as_str() {
        "aarch64-unknown-linux-gnu" | "aarch64-linux-gnu" => (
            "liblitert-lm.so",
            "lib/linux_arm64",
            "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/linux_arm64_liblitert-lm.so",
        ),
        "x86_64-linux-gnu" | "x86_64-unknown-linux-gnu" => (
            "liblitert-lm.so",
            "lib/linux_x86_64",
            "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/linux_x86_64_liblitert-lm.so",
        ),
        "aarch64-linux-android" => (
            "liblitert-lm.so",
            "lib/android_arm64",
            "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/android_arm64_liblitert-lm.so",
        ),
        "x86_64-linux-android" => (
            "liblitert-lm.so",
            "lib/android_x86_64",
            "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/android_x86_64_liblitert-lm.so",
        ),
        "aarch64-apple-darwin" => (
            "liblitert-lm.dylib",
            "lib/macos_arm64",
            "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/liblitert-lm.dylib",
        ),
        "x86_64-pc-windows-msvc" => (
            "litert-lm.dll",
            "lib/windows_x86_64/bin",
            "https://github.com/kiamazi/LiteRT-LM-prebuilts/releases/download/v0.16.0/windows_x86_64_litert-lm.dll",
        ),
        _ => panic!("Unsupported target: {}", target),
    };

    // Create subdirectory structure in OUT_DIR
    let lib_subdir = out_dir.join(subdir);
    std::fs::create_dir_all(&lib_subdir)
        .unwrap_or_else(|e| panic!("Failed to create directory {:?}: {}", lib_subdir, e));

    let lib_path = lib_subdir.join(lib_filename);

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
        configure_linking(&lib_subdir, lib_filename);
        generate_bindings(&manifest_dir);
        return;
    }

    // Priority 3: Download from GitHub as fallback
    println!(
        "cargo:warning=No prebuilt library found, downloading from GitHub"
    );
    download_file(url, &lib_path);

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

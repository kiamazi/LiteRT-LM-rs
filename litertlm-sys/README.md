# litertlm-sys

Raw, unsafe `bindgen`-generated FFI bindings to Google's [LiteRT LM](https://github.com/google-ai-edge/LiteRT-LM)
C API **1.0.0** (`engine.h`, `conversation.h`, `embedding_engine.h`,
`model_info.h`, `experimental.h`, `error_reporter.h`, `api_export.h`).

This crate is a thin `-sys` layer: every function is `unsafe`, pointer-based,
and a 1:1 mirror of the C API. **Most users want
[`litertlm-rs`](https://crates.io/crates/litertlm-rs) instead** — a safe,
idiomatic wrapper built on top of this crate. Reach for `litertlm-sys`
directly only if you need functionality the safe wrapper doesn't expose yet,
or you're writing your own abstraction.

## What this crate does at build time

`build.rs` gets a copy of `liblitert-lm.so` (`.dylib` on macOS, `litert-lm.dll`
on Windows) in this order:

1. `LITERT_LM_LIB_DIR` environment variable, if set. It must point at the
   *package root* of the official prebuilt, i.e. the directory that contains
   `lib/<platform>/liblitert-lm.so` (not the platform directory itself).
2. A `prebuilt/lib/<platform>/` directory next to this crate's `Cargo.toml`
   (used for local development of this repo).
3. Otherwise, downloads the matching prebuilt for your target from
   [LiteRT-LM-prebuilts](https://github.com/kiamazi/LiteRT-LM-prebuilts/releases)
   (an independent mirror of
   [LiteRT-LM's GitHub releases](https://github.com/google-ai-edge/LiteRT-LM/releases)).

> [!NOTE]
> Google publishes a single `.zip` per LiteRT-LM release containing shared
> libraries for _every_ supported platform. Downloading and unzipping that
> entire archive just to extract the one `.so`/`.dylib`/`.dll` your build
> actually needs is wasteful — especially in a `build.rs` script that runs
> on every `cargo build` or CI job.

It then runs `bindgen` over `wrapper.h` (which includes every public header in
`prebuilt/include/`) to generate the raw bindings, allow-listing the
`litert_lm_*` functions, `LiteRtLm*` types and `LITERT_LM_*` constants (such as
`LITERT_LM_C_API_VERSION_MAJOR`).

> [!IMPORTANT]
> The C API 1.0.0 is ABI-incompatible with 0.x. The headers in
> `prebuilt/include/` and the library you link must come from the same
> package. The download fallback (step 3) fetches the 1.0.0 libraries from
> the `LiteRT-LM-prebuilts` mirror (tag `v0.19.1`) and verifies their SHA-256.

**System requirement:** `bindgen` needs `libclang` available at build time
(a developer-machine dependency only — it isn't needed by the compiled
binary):

```bash
# Debian/Ubuntu
sudo apt install libclang-dev clang
# Fedora
sudo dnf install clang-devel
# macOS
brew install llvm
```

See [`prebuilt/README.md`](prebuilt/README.md) in this crate for the exact
directory layout expected under `prebuilt/`, and the full repo README —
[litert-lm-rs](https://github.com/kiamazi/LiteRT-LM-rs) — for end-to-end
setup, including how the native library gets found again at _runtime_
(that part is handled by `litertlm-rs`'s build script, not this crate's).

## License

Apache-2.0.

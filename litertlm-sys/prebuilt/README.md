# prebuilt/

This holds Google's official C API prebuilt package for LiteRT-LM (C API 1.0.0):
https://github.com/google-ai-edge/LiteRT-LM/releases

The seven headers in `include/` (`api_export.h`, `conversation.h`,
`embedding_engine.h`, `engine.h`, `error_reporter.h`, `experimental.h`,
`model_info.h`) are already here (from the package you downloaded). **You still need to copy the `lib/` directory from
that same downloaded package into `prebuilt/lib/`**, so the layout under
this directory matches exactly what you extracted:

```
litertlm-sys/prebuilt/
├── include/
│   ├── api_export.h
│   ├── conversation.h
│   ├── embedding_engine.h
│   ├── engine.h
│   ├── error_reporter.h
│   ├── experimental.h
│   └── model_info.h
└── lib/
    ├── linux_arm64/
    │   └── liblitert-lm.so
    ├── linux_x86_64/
    │   └── liblitert-lm.so
    ├── macos_arm64/
    │   └── liblitert-lm.dylib
    ├── windows_x86_64/
    │   ├── bin/
    │   │   └── litert-lm.dll
    │   └── lib/
    │       └── litert-lm.lib
    └── android_arm64/ , android_x86_64/
        └── liblitert-lm.so
```

`build.rs` picks the right subdirectory automatically based on your build
target (the `TARGET` triple cargo passes it). Only the subdirectory
matching your actual platform needs to be populated — e.g. on Linux x86_64
you only need `lib/linux_x86_64/` to have the file in it.

Override the whole lookup with `LITERT_LM_LIB_DIR=/path/to/package` if you'd
rather keep the prebuilt package somewhere else entirely. It is the package
root — the directory that contains `lib/<platform>/...` — so
`$LITERT_LM_LIB_DIR/lib/linux_x86_64/liblitert-lm.so` must exist.

## Runtime

Unlike the old locally-built `libengine.so`, this is a single self-contained
library — no sibling plugin `.so` files to also copy around, no `patchelf`
step needed. Just make sure `liblitert-lm.so` (or the `.dylib`/`.dll`)
ends up next to your compiled binary. If you're depending on `litertlm-rs`
(rather than `litertlm-sys` directly), its `build.rs` does this for you
automatically — see the root repo README's "Runtime linking" section.

#!/bin/bash
set -euxo pipefail
dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

# Every query is a `sqlx` macro; `SQLX_OFFLINE` makes the cross-build read the
# committed `.sqlx` cache instead of wanting a database.
(cd "$dir" &&
    SQLX_OFFLINE=true CC="/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang" AR="/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar" cargo build --target aarch64-linux-android --release --config 'target.aarch64-linux-android.linker = "/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang"')

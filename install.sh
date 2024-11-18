#!/bin/bash
set -euxo pipefail
dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)

cargo install --path "$dir"

(cd "$dir" &&
    CC="/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang" AR="/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar" cargo build --target aarch64-linux-android --release --config 'target.aarch64-linux-android.linker = "/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android35-clang"')

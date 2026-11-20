#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

sdk=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}
: "${sdk:?Set ANDROID_SDK_ROOT to your Android SDK}"
ndk=${ANDROID_NDK_HOME:-"$sdk/ndk/28.2.13676358"}
tools=${ANDROID_BUILD_TOOLS:-"$sdk/build-tools/37.0.0"}
jar_file=${ANDROID_PLATFORM_JAR:-"$sdk/platforms/android-37.0/android.jar"}

case "$(uname -s)-$(uname -m)" in
    Darwin-*) host=darwin-x86_64 ;;
    Linux-x86_64) host=linux-x86_64 ;;
    *) echo "Run this script on macOS or x86_64 Linux." >&2; exit 1 ;;
esac
llvm="$ndk/toolchains/llvm/prebuilt/$host/bin"
if [[ ! -f "$jar_file" || ! -x "$tools/aapt2" || ! -x "$llvm/llvm-readelf" ]]; then
    echo "Install Android SDK 37.0, Build Tools 37.0.0 and NDK r28c." >&2
    exit 1
fi

if [[ $# -eq 0 ]]; then set -- arm64-v8a x86_64; fi
out="$PWD/target/android"
mkdir -p "$out"
build=$(mktemp -d "$out/apk.XXXXXXXX")
trap 'rm -rf -- "$build"' EXIT
mkdir -p "$build/classes" "$build/dex" "$build/lib" "$PWD/dist/android"
export CARGO_TARGET_DIR="$out/rust"

for abi in "$@"; do
    case "$abi" in
        arm64-v8a) target=aarch64-linux-android; upper=AARCH64_LINUX_ANDROID; lower=aarch64_linux_android ;;
        x86_64) target=x86_64-linux-android; upper=X86_64_LINUX_ANDROID; lower=x86_64_linux_android ;;
        *) echo "Unknown ABI: $abi" >&2; exit 1 ;;
    esac
    cc="$llvm/${target}26-clang"
    env "CARGO_TARGET_${upper}_LINKER=$cc" "CC_${lower}=$cc" "AR_${lower}=$llvm/llvm-ar" \
        cargo rustc --locked --release --lib --target "$target" --crate-type cdylib
    so="$build/lib/$abi/libtermian_native.so"
    mkdir -p "$(dirname "$so")"
    "$llvm/llvm-strip" -o "$so" "$CARGO_TARGET_DIR/$target/release/libtermian_native.so"
    "$llvm/llvm-readelf" --program-headers "$so" | awk '
        $1 == "LOAD" {
            seen = 1
            alignment = tolower($NF)
            sub(/^0x0*/, "", alignment)
            if (length(alignment) < 4 || (length(alignment) == 4 && alignment !~ /^[4-9a-f]/)) bad = 1
        }
        END { if (!seen || bad) exit 1 }
    ' || { echo "$abi: native library load segments are not aligned for 16 KiB pages." >&2; exit 1; }
done

javac -encoding UTF-8 --release 8 -classpath "$jar_file" -d "$build/classes" android/src/app/termian/mobile/*.java
jar cf "$build/classes.jar" -C "$build/classes" .
"$tools/d8" --release --min-api 26 --lib "$jar_file" --output "$build/dex" "$build/classes.jar"
mkdir -p "$build/res/drawable" "$build/assets/fonts"
cp -R android/res/. "$build/res/"
cp assets/fonts/CircularXX-*.ttf "$build/assets/fonts/"
python3 scripts/android-icons.py "$build/res/drawable"
"$tools/aapt2" compile --dir "$build/res" -o "$build/resources.zip"
"$tools/aapt2" link -I "$jar_file" --manifest android/AndroidManifest.xml -A "$build/assets" \
    --min-sdk-version 26 --target-sdk-version 37 -o "$build/raw.apk" "$build/resources.zip"
(cd "$build"; cp dex/classes.dex .; zip -q -0 raw.apk classes.dex; zip -q -0 -r raw.apk lib)
"$tools/zipalign" -P 16 -f 4 "$build/raw.apk" "$build/aligned.apk"

key="$out/debug.keystore"
if [[ ! -f "$key" ]]; then
    keytool -genkeypair -keystore "$key" -storepass android -keypass android -alias androiddebugkey \
        -dname 'CN=Android Debug,O=Android,C=US' -keyalg RSA -keysize 2048 -validity 10000
fi
"$tools/apksigner" sign --ks "$key" --ks-pass pass:android --ks-key-alias androiddebugkey \
    --out "$build/termian-debug.apk" "$build/aligned.apk"
"$tools/apksigner" verify "$build/termian-debug.apk"
"$tools/zipalign" -c -P 16 4 "$build/termian-debug.apk"
mv "$build/termian-debug.apk" dist/android/termian-debug.apk
echo "dist/android/termian-debug.apk"

#!/bin/bash

set -e

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

mode=debug

if [ "$1" == "release" ]; then
	mode=release
fi

BASE=`dirname "$0"`
PROJECT_BASE=`realpath $BASE/../`
BUILD_DIR="$PROJECT_BASE/build/android/$mode"
HOST_OS=`uname -s | tr "[:upper:]" "[:lower:]"`
HOST_ARCH=`uname -m | tr "[:upper:]" "[:lower:]"`
if [ "${HOST_OS}" == "darwin" ] && [ "${HOST_ARCH}" == "arm64" ]; then
    # NDK DOESNT HAVE AN ARM64 TOOLCHAIN ON DARWIN
    # WE USE x86-64 WITH ROSETTA INSTEAD
    HOST_ARCH=x86_64
fi

api=30
ndk_version=22.1.7171670
android_tools="$NDK_HOME/$ndk_version/toolchains/llvm/prebuilt/$HOST_OS-$HOST_ARCH/bin"

CC_preifx=${target}${api}
AR_preifx=${target}${api}

for target in x86_64-linux-android aarch64-linux-android; do
    case $target in
        'x86_64-linux-android')
            export CC_x86_64_linux_android="$android_tools/${target}${api}-clang"
            export AR_x86_64_linux_android="$android_tools/${target}-ar"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_AR="$android_tools/$target-ar"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$android_tools/${target}${api}-clang"
            export PATH="$NDK_HOME/$ndk_version/toolchains/llvm/prebuilt/$HOST_OS-$HOST_ARCH/bin/":$PATH
            mkdir -p "$BUILD_DIR/jni/x86_64/"
            case $mode in
              'release')
                cargo build --target $target --manifest-path "$BASE/Cargo.toml" --release
                cp "$PROJECT_BASE/target/$target/release/libleaf.so" "$BUILD_DIR/jni/x86_64/"
                ;;
              *)
                cargo build --target $target --manifest-path "$BASE/Cargo.toml"
                cp "$PROJECT_BASE/target/$target/debug/libleaf.so" "$BUILD_DIR/jni/x86_64/"
                ;;
            esac
            ;;
        'aarch64-linux-android')
            export CC_aarch64_linux_android="$android_tools/${target}${api}-clang"
            export AR_aarch64_linux_android="$android_tools/${target}-ar"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="$android_tools/$target-ar"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$android_tools/${target}${api}-clang"
            export PATH="$NDK_HOME/$ndk_version/toolchains/llvm/prebuilt/$HOST_OS-$HOST_ARCH/bin/":$PATH
            mkdir -p "$BUILD_DIR/jni/arm64-v8a/"
            case $mode in
              'release')
                cargo build --target $target --manifest-path "$BASE/Cargo.toml" --release
                cp "$PROJECT_BASE/target/$target/release/libleaf.so" "$BUILD_DIR/jni/arm64-v8a/"
                ;;
              *)
                cargo build --target $target --manifest-path "$BASE/Cargo.toml"
                cp "$PROJECT_BASE/target/$target/debug/libleaf.so" "$BUILD_DIR/jni/arm64-v8a/"
                ;;
            esac
			      ;;
        *)
            echo "Unknown target $target"
            ;;
    esac
done

javac $BASE/Leaf.java -d $BUILD_DIR
pushd $BUILD_DIR > /dev/null
jar cvf classes.jar leaf/Leaf.class
popd > /dev/null

pushd $BUILD_DIR > /dev/null
cat > AndroidManifest.xml <<EOF
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="rust.boundary.rustjni">
<uses-sdk android:minSdkVersion="15"/></manifest>
EOF
zip -r leaf.aar classes.jar AndroidManifest.xml jni
popd > /dev/null
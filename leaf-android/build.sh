#!/bin/bash

mode=debug

if [ "$1" == "release" ]; then
	mode=release
fi

BASE=`dirname "$0"`
BUILD_DIR="$BASE/build"
HOST_OS=`uname -s | tr "[:upper:]" "[:lower:]"`
HOST_ARCH=`uname -m | tr "[:upper:]" "[:lower:]"`
if [ "${HOST_OS}" == "darwin" ] && [ "${HOST_ARCH}" == "arm64" ]; then
    # NDK DOESNT HAVE AN ARM64 TOOLCHAIN ON DARWIN
    # WE USE x86-64 WITH ROSETTA INSTEAD
    HOST_ARCH=x86_64
fi

android_tools="$NDK_HOME/toolchains/llvm/prebuilt/$HOST_OS-$HOST_ARCH/bin"
api=31

for target in x86_64-linux-android aarch64-linux-android; do
    case $target in
        'x86_64-linux-android')
            export CC_x86_64_linux_android="$android_tools/${target}${api}-clang"
            export AR_x86_64_linux_android="$android_tools/llvm-ar"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_AR="$android_tools/llvm-ar"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$android_tools/${target}${api}-clang"
            export PATH="$NDK_HOME/toolchains/llvm/prebuilt/$HOST_OS-$HOST_ARCH/bin/":$PATH
            mkdir -p "$BUILD_DIR/jniLibs/x86_64/"
			case $mode in
				'release')
#				  cargo build --target $target --manifest-path "$BASE/Cargo.toml" --no-default-features --features "leaf/default-openssl" --release
					cargo build --target $target --manifest-path "$BASE/Cargo.toml" --release
					cp "$BASE/../target/$target/release/libleaf.so" "$BUILD_DIR/jniLibs/x86_64/"
					;;
				*)
					cargo build --target $target --manifest-path "$BASE/Cargo.toml"
					cp "$BASE/../target/$target/debug/libleaf.so" "$BUILD_DIR/jniLibs/x86_64/"
					;;
			esac
            ;;
        'aarch64-linux-android')
            export CC_aarch64_linux_android="$android_tools/${target}${api}-clang"
            export AR_aarch64_linux_android="$android_tools/llvm-ar"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="$android_tools/llvm-ar"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$android_tools/${target}${api}-clang"
            export PATH="$NDK_HOME/toolchains/llvm/prebuilt/$HOST_OS-$HOST_ARCH/bin/":$PATH
            mkdir -p "$BUILD_DIR/jniLibs/arm64-v8a/"
			case $mode in
				'release')
					cargo build --target $target --manifest-path "$BASE/Cargo.toml" --release
					cp "$BASE/../target/$target/release/libleaf.so" "$BUILD_DIR/jniLibs/arm64-v8a/"
					;;
				*)
					cargo build --target $target --manifest-path "$BASE/Cargo.toml"
					cp "$BASE/target/$target/debug/libleaf.so" "$BUILD_DIR/jniLibs/arm64-v8a/"
					;;
			esac
			;;
        *)
            echo "Unknown target $target"
            ;;
    esac
done
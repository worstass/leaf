#!/usr/bin/env bash

set -eEu

realpath() {
  [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

BASE=`dirname "$0"`
PROJECT_BASE=`realpath $BASE/..`
if [ -n "$CONFIGURATION" ]; then CONFIG=$CONFIGURATION; else CONFIG="Debug"; fi
OUTPUT_DIR="${PROJECT_BASE}/build/apple/${CONFIG}"
mkdir -p ${OUTPUT_DIR}

if [ "$CONFIG" == "Release" ]; then
  cargo_build_flags="--release"
  build_type="release"
elif [ "$CONFIG" == "Debug" ]; then
  cargo_build_flags=""
  build_type="debug"
else
  echo "Unknown configuration type"
fi

# iOS
echo -e "Building for iOS [1/5]"
cd $PROJECT_BASE/leaf-ffi && cargo build $cargo_build_flags --target "aarch64-apple-ios"

# MacOS
echo -e "Building for macOS (Apple Silicon) [2/5]"
cd $PROJECT_BASE/leaf-ffi && cargo build $cargo_build_flags --target "aarch64-apple-darwin"
echo -e "Building for macOS (Intel) [3/5]"
cd $PROJECT_BASE/leaf-ffi && cargo build $cargo_build_flags --target "x86_64-apple-darwin"

# iOS Simulator
echo -e "Building for iOS Simulator (Apple Silicon) [4/5]"
cd $PROJECT_BASE/leaf-ffi && cargo build $cargo_build_flags --target "aarch64-apple-ios-sim"
echo -e "Building for iOS Simulator (Intel) [5/5]"
cd $PROJECT_BASE/leaf-ffi && cargo build $cargo_build_flags --target "x86_64-apple-ios"

echo -e "\nCreating XCFramework"
# MacOS
lipo -create \
  "${PROJECT_BASE}/target/x86_64-apple-darwin/${build_type}/libleaf.a" \
  "${PROJECT_BASE}/target/aarch64-apple-darwin/${build_type}/libleaf.a" \
  -output "${OUTPUT_DIR}/libleaf_macos.a"
lipo -info "${OUTPUT_DIR}/libleaf_macos.a"

# iOS Simulator
lipo -create \
  "${PROJECT_BASE}/target/x86_64-apple-ios/${build_type}/libleaf.a" \
  "${PROJECT_BASE}/target/aarch64-apple-ios-sim/${build_type}/libleaf.a" \
  -output "${OUTPUT_DIR}/libleaf_iossimulator.a"
lipo -info "${OUTPUT_DIR}/libleaf_iossimulator.a"

# iOS
lipo -create \
  "${PROJECT_BASE}/target/aarch64-apple-ios/${build_type}/libleaf.a" \
  -output "${OUTPUT_DIR}/libleaf_ios.a"
lipo -info "${OUTPUT_DIR}/libleaf_ios.a"

mkdir -p "${OUTPUT_DIR}/headers"
cp "${PROJECT_BASE}/leaf-ffi/leaf-c.h" "${OUTPUT_DIR}/headers/leaf.h"

if [ -d "${OUTPUT_DIR}/leaf.xcframework" ]; then rm -rf "${OUTPUT_DIR}/leaf.xcframework"; fi

export BUILD_LIBRARY_FOR_DISTRIBUTION=YES
xcodebuild -create-xcframework \
  -library "${OUTPUT_DIR}/libleaf_macos.a" \
  -headers "${OUTPUT_DIR}/headers" \
  -library "${OUTPUT_DIR}/libleaf_iossimulator.a" \
  -headers "${OUTPUT_DIR}/headers" \
  -library "${OUTPUT_DIR}/libleaf_ios.a" \
  -headers "${OUTPUT_DIR}/headers" \
  -output "${OUTPUT_DIR}/leaf.xcframework"
#!/usr/bin/env bash

set -e

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

scheme=$1 # macos or ios
if [ -n "$CONFIGURATION" ]; then CONFIG=$CONFIGURATION; else CONFIG="Debug"; fi
BASE=`dirname "$0"`
PROJECT_BASE=`realpath $BASE/..`

macosx_platform_targets="aarch64-apple-darwin x86_64-apple-darwin"
iphoneos_platform_targets="aarch64-apple-ios"
iphonesimulator_platform_targets="x86_64-apple-ios aarch64-apple-ios-sim"
maccatalyst_platform_targets="x86_64-apple-ios-macabi aarch64-apple-ios-macabi"

if [ "$scheme" == "macos" ]; then
  targets="$macosx_platform_targets"
elif [ "$scheme" == "ios" ]; then
  targets="$iphoneos_platform_targets $iphonesimulator_platform_targets $maccatalyst_platform_targets"
else
  echo "Unknown scheme type: $scheme"
  exit
fi

if [ "$CONFIG" == "Release" ]; then
  cargo_build_flags="--release"
  build_type="release"
elif [ "$CONFIG" == "Debug" ]; then
  cargo_build_flags=""
  build_type="debug"
else
  echo "Unknown configuration type"
fi

for target in $targets; do
  if [[ $target == *macabi ]]; then flags="-Z build-std=panic_abort,std"; else flags=""; fi
  echo "Build for target: ${target}"
  cd $PROJECT_BASE/leaf-ffi && cargo build $flags --target $target $cargo_build_flags
done

if [ "$scheme" == "macos" ]; then
  targets="$macosx_platform_targets"
  for subfix in 'a' 'dylib'; do
    for platform in macosx; do
      libs=""
      for target in $targets; do
        libs="$libs $PROJECT_BASE/target/$target/$build_type/libleaf.$subfix"
      done
      lib_output="$PROJECT_BASE/build/apple/$CONFIG-$platform"
      mkdir -p $lib_output
      lipo -create $libs -output $lib_output/libleaf.$subfix
      lipo -info $lib_output/libleaf.$subfix
    done
  done
elif [ "$scheme" == "ios" ]; then
  for subfix in 'a' 'dylib'; do
    for platform in iphoneos iphonesimulator maccatalyst; do
      tgs=${platform}_platform_targets
      targets=${!tgs}
      libs=""
      for target in $targets; do
        libs="$libs $PROJECT_BASE/target/$target/$build_type/libleaf.$subfix"
      done
      lib_output="$PROJECT_BASE/build/apple/$CONFIG-$platform"
      mkdir -p $lib_output
      lipo -create $libs -output $lib_output/libleaf.$subfix
      lipo -info $lib_output/libleaf.$subfix
    done
  done
else
  echo "Unknown scheme type: $scheme"
  exit
fi
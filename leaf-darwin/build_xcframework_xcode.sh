#!/usr/bin/env bash

set -eEu

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

BASE=`dirname "$0"`
PROJECT_BASE=`realpath $BASE/..`
if [ -n "$CONFIGURATION" ]; then CONFIG=$CONFIGURATION; else CONFIG="Debug"; fi
if [ "$CONFIG" == "Release" ]; then
  build_type="Release"
elif [ "$CONFIG" == "Debug" ]; then
  build_type="Debug"
else
  echo "Unknown configuration type"
fi

ARCHIVES_DIR="$PROJECT_BASE/build/apple/$build_type/archives"
mkdir -p $ARCHIVES_DIR

MACOS_ARCHIVE="$ARCHIVES_DIR/leaf.macos.xcarchive"
IOS_ARCHIVE="$ARCHIVES_DIR/leaf.ios.xcarchive"
IOS_SIM_ARCHIVE="$ARCHIVES_DIR/leaf.iossim.xcarchive"

xcodebuild archive -scheme leaf-macos -destination "generic/platform=OS X" -archivePath $MACOS_ARCHIVE SKIP_INSTALL=NO BUILD_LIBRARY_FOR_DISTRIBUTION=YES
xcodebuild archive -scheme leaf-ios -destination "generic/platform=iOS" -archivePath $IOS_ARCHIVE SKIP_INSTALL=NO BUILD_LIBRARY_FOR_DISTRIBUTION=YES
xcodebuild archive -scheme leaf-ios -destination "generic/platform=iOS Simulator" -archivePath $IOS_SIM_ARCHIVE SKIP_INSTALL=NO BUILD_LIBRARY_FOR_DISTRIBUTION=YES

if [ -d "$PROJECT_BASE/build/apple/leaf.xcframework" ]; then rm -rf "$PROJECT_BASE/build/apple/leaf.xcframework"; fi

xcodebuild -create-xcframework \
    -framework "$MACOS_ARCHIVE/Products/Library/Frameworks/leaf.framework" \
    -framework "$IOS_ARCHIVE/Products/Library/Frameworks/leaf.framework" \
    -framework "$IOS_SIM_ARCHIVE/Products/Library/Frameworks/leaf.framework" \
    -output "$PROJECT_BASE/build/apple/leaf.xcframework"

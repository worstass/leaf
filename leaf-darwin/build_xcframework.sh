#!/usr/bin/env bash

set -e

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

BASE=`dirname "$0"`
PROJECT_BASE=`realpath $BASE/..`
ARCHIVES_DIR="$PROJECT_BASE/build/apple/$BUILD_TYPE/archives"
mkdir -p $ARCHIVES_DIR

MACOS_ARCHIVE="$ARCHIVES_DIR/leaf-macos.macos.xcarchive"
IOS_ARCHIVE="$ARCHIVES_DIR/leaf-ios.ios.xcarchive"
IOS_SIM_ARCHIVE="$ARCHIVES_DIR/leaf-ios.iossim.xcarchive"
MAC_CATALYST_ARCHIVE="$ARCHIVES_DIR/leaf-ios.macos.xcarchive"

xcodebuild archive -scheme leaf-macos -archivePath $MACOS_ARCHIVE SKIP_INSTALL=NO
xcodebuild archive -scheme leaf-ios -destination "generic/platform=iOS" -archivePath $IOS_ARCHIVE SKIP_INSTALL=NO
xcodebuild archive -scheme leaf-ios -destination "generic/platform=iOS Simulator" -archivePath $IOS_SIM_ARCHIVE SKIP_INSTALL=NO
xcodebuild archive -scheme leaf-ios -destination "generic/platform=macOS,variant=Mac Catalyst" -archivePath $MAC_CATALYST_ARCHIVE SKIP_INSTALL=NO
xcodebuild -create-xcframework \
    -framework "$MACOS_ARCHIVE/Products/Library/Frameworks/leaf.framework" \
    -framework "$IOS_ARCHIVE/Products/Library/Frameworks/leaf.framework" \
    -framework "$IOS_SIM_ARCHIVE/Products/Library/Frameworks/leaf.framework" \
    -framework "$MAC_CATALYST_ARCHIVE/Products/Library/Frameworks/leaf.framework" \
    -output "$PROJECT_BASE/build/apple/leaf.xcframework"

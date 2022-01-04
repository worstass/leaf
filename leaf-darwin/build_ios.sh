#!/usr/bin/env bash

set -e
BASE=`dirname "$0"`
#BUILD_DIR="$BASE/build"
#mkdir -p $BUILD_DIR
#cd $BUILD_DIR
cd $BASE/../leaf-ffi

#aarch64-apple-darwin
#aarch64-apple-ios

#x86_64-apple-darwin
#x86_64-apple-ios

cargo build --target x86_64-apple-ios

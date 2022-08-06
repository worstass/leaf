#!/usr/bin/env bash

set -e

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

BASE=$(dirname "$0")
PROJECT_BASE=$(realpath "$BASE"/../../)
BUILD_DIR="$PROJECT_BASE/build/java"
if [ -d "$BUILD_DIR" ]; then rm -rf "$BUILD_DIR"; fi
mkdir -p $BUILD_DIR
pushd $PROJECT_BASE/leaf-android && cargo build --release && popd

cp $PROJECT_BASE/target/release/libleaf.dylib $BUILD_DIR
CLASSPATH=$BUILD_DIR
javac $BASE/../Leaf.java  -cp $CLASSPATH -d "$BUILD_DIR"
javac $BASE/Runner.java -cp $CLASSPATH -d "$BUILD_DIR"

cd $BUILD_DIR && java -cp $CLASSPATH leaf.Runner
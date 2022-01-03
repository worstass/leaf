#!/usr/bin/env bash

set -x
BASE=`dirname "$0"`
BUILD_DIR="$BASE/build"
mkdir -p $BUILD_DIR
cd $BUILD_DIR
mkdir -p {ios,macos}/leaf.framework/{Headers,Versions}
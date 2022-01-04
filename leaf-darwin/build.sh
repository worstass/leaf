#!/usr/bin/env bash

set -e

realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

BUILD_TYPE="${BUILD_TYPE:=debug}"
BASE=`dirname "$0"`
PROJECT_BASE=`realpath $BASE/..`
BUILD_DIR="$PROJECT_BASE/build/apple/$BUILD_TYPE/$PLATFORM_NAME"
mkdir -p $BUILD_DIR

if [ "$BUILD_TYPE" == "release" ]; then
  build_type="--release"
else
  build_type=""
fi

#echo $BUILD_TYPE
#echo $PLATFORM_NAME
#echo $ARCHS

if [ "$PLATFORM_NAME" == "macosx" ]; then
  host_os="darwin"
elif [ "$PLATFORM_NAME" == "ios" ]; then
  host_os="ios"
else
  echo "Unknown host_os"
fi

echo $host_os

libs=""
for arch in $ARCHS; do
   case $arch in
     'x86_64')
         cargo build --target x86_64-apple-$host_os $build_type
         libs="$libs $PROJECT_BASE/target/x86_64-apple-$host_os/$BUILD_TYPE/libleaf.a"
         ;;
     'arm64')
         cargo build --target aarch64-apple-$host_os $build_type
         libs="$libs $PROJECT_BASE/target/aarch64-apple-$host_os/$BUILD_TYPE/libleaf.a"
         ;;
     *)
         echo "Unknown target $arch"
         ;;
   esac
done

lipo -create $libs -output $BUILD_DIR/libleaf.a
lipo -info $BUILD_DIR/libleaf.a

#!/usr/bin/env bash

pushd ../leaf-ffi > /dev/null || exit
cbindgen --config cbindgen.toml  --crate leaf-ffi --output ../leaf-darwin/leaf.h
popd > /dev/null || exit


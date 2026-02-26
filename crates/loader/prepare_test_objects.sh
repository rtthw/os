#!/bin/sh

set -ex

rustc tests/input/add_one.rs \
    --out-dir=tests/output \
    --crate-type=lib \
    --emit=link,obj \
    -C panic=abort \
    -C relocation-model=static \
    -Z share-generics=no

rustc tests/input/depends_on_add_one.rs \
    --out-dir=tests/output \
    --crate-type=lib \
    --emit=obj \
    -L tests/output \
    -l add_one \
    -C panic=abort \
    -C relocation-model=static \
    -Z share-generics=no

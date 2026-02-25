#!/bin/sh


SCRIPT_DIR=${0%/*}
INPUT_DIR="tests/input"
OUTPUT_DIR="tests/output"

mkdir -p $OUTPUT_DIR

for filename in $(ls $INPUT_DIR); do
    path="$INPUT_DIR/$filename"
    echo "Compiling '$filename'..."
    rustc \
        --out-dir=$OUTPUT_DIR \
        --crate-type=rlib \
        --emit=obj \
        -Cpanic=abort \
        -Zshare-generics=no \
        $path
    echo "\t...OK"
done

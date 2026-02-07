#!/bin/sh


set -ex

cd ../abi
    cargo build

cd ../emulator
    cargo run

#!/bin/bash
source ./config.sh

TASK=macswap

if [ $# -ge 1 ]; then
    TASK=$1
fi

echo $TASK

pushd $TASK
env RUST_BACKTRACE=1 cargo run --target x86_64-fortanix-unknown-sgx
popd

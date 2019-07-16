#!/bin/bash

MODE=debug
TASK=macswap

if [ $# -ge 1 ]; then
    TASK=$1
fi
echo $TASK

# Build custom runner
pushd runner
if [ "$MODE" == "debug" ]; then
    cargo +nightly build
else
    cargo +nightly build --release
fi
popd

# Build enclave APP
pushd examples/$TASK
if [ "$MODE" == "debug" ]; then
    cargo +nightly build --target=x86_64-fortanix-unknown-sgx
else
    cargo +nightly build --target=x86_64-fortanix-unknown-sgx --release
fi
popd

# Convert the APP
ftxsgx-elf2sgxs target/x86_64-fortanix-unknown-sgx/$MODE/$TASK --heap-size 0x20000 --stack-size 0x20000 --threads 1 --debug

# Execute
runner/target/$MODE/runner target/x86_64-fortanix-unknown-sgx/$MODE/${TASK}.sgxs

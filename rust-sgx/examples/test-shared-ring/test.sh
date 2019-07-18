#!/bin/bash

BASE_DIR="/home/yangz/SafeBricks"
PORT_OPTIONS="dpdk:eth_pcap0,rx_pcap=/home/yangz/traffic/caida18_real.pcap,tx_pcap=/tmp/out.pcap"

if [[ -z ${CARGO_INCREMENTAL} ]] || [[ $CARGO_INCREMENTAL = false ]] || [[ $CARGO_INCREMENTAL = 0 ]]; then
    export CARGO_INCREMENTAL="CARGO_INCREMENTAL=0 "
fi

if [[ -z ${RUST_BACKTRACE} ]] || [[ RUST_BACKTRACE = true ]] || [[ RUST_BACKTRACE = 1 ]]; then
    export RUST_BACKTRACE="RUST_BACKTRACE=1 "
fi

echo "Current Cargo Incremental Setting: ${CARGO_INCREMENTAL}"
echo "Current Rust Backtrace Setting: ${RUST_BACKTRACE}"

DPDK_VER=17.08
DPDK_HOME="/opt/dpdk/dpdk-stable-${DPDK_VER}"
DPDK_LD_PATH="${DPDK_HOME}/build/lib"
DPDK_CONFIG_FILE=${DPDK_CONFIG_FILE-"${DPDK_HOME}/config/common_linuxapp"}

NATIVE_LIB_PATH="${BASE_DIR}/native"
export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

MODE=debug
TASK=app

if [ $# -ge 1 ]; then
    TASK=$1
fi
echo $TASK

native () {
    make -j $proc -C $BASE_DIR/native
    make -C $BASE_DIR/native install
}

native

# Build custom runner
pushd runner
if [ "$MODE" == "debug" ]; then
    cargo +nightly build
else
    cargo +nightly build --release
fi
popd

# Build enclave APP
pushd $TASK
if [ "$MODE" == "debug" ]; then
    cargo +nightly build --target=x86_64-fortanix-unknown-sgx
else
    cargo +nightly build --target=x86_64-fortanix-unknown-sgx --release
fi
popd

# Convert the APP
ftxsgx-elf2sgxs app/target/x86_64-fortanix-unknown-sgx/$MODE/$TASK --heap-size 0x20000 --stack-size 0x20000 --threads 2 --debug

# Execute
export PATH="${BIN_DIR}:${PATH}"
export LD_LIBRARY_PATH="${NATIVE_LIB_PATH}:${DPDK_LD_PATH}:${LD_LIBRARY_PATH}"
# echo "sudo env PATH=\"$PATH\" LD_LIBRARY_PATH=\"$LD_LIBRARY_PATH\" LD_PRELOAD=\"$LD_PRELOAD\" $executable \"$@\""

env PATH="$PATH" LD_LIBRARY_PATH="$LD_LIBRARY_PATH" LD_PRELOAD="$LD_PRELOAD" \
    RUST_BACKTRACE=1 runner/target/$MODE/runner -s app/target/x86_64-fortanix-unknown-sgx/$MODE/${TASK}.sgxs -p $PORT_OPTIONS -c 0

#!/bin/bash
set -e


PORT_OPTIONS="dpdk:eth_pcap0,rx_pcap=../traffic/caida18_real.pcap,tx_pcap=/tmp/out.pcap"
# PORT_OPTIONS="0000:02:00.0"
MODE=debug
TASK=macswap
QUEUE="single"

BASE_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd)"
BUILD_SCRIPT=$( basename "$0" )

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

native () {
    make -j $proc -C $BASE_DIR/native
    make -C $BASE_DIR/native install
}

native

# Build custom runner
pushd dpdkIO
if [ "$MODE" == "debug" ]; then
    cargo +nightly build
else
    cargo +nightly build --release
fi
popd

# Execute
export PATH="${BIN_DIR}:${PATH}"
export LD_LIBRARY_PATH="${NATIVE_LIB_PATH}:${DPDK_LD_PATH}:${LD_LIBRARY_PATH}"
# echo "sudo env PATH=\"$PATH\" LD_LIBRARY_PATH=\"$LD_LIBRARY_PATH\" LD_PRELOAD=\"$LD_PRELOAD\" $executable \"$@\""

if [ "$QUEUE" == "single" ]; then
    env PATH="$PATH" LD_LIBRARY_PATH="$LD_LIBRARY_PATH" LD_PRELOAD="$LD_PRELOAD" \
    RUST_BACKTRACE=1 target/$MODE/dpdkIO -p $PORT_OPTIONS -c 0
else
    env PATH="$PATH" LD_LIBRARY_PATH="$LD_LIBRARY_PATH" LD_PRELOAD="$LD_PRELOAD" \
    RUST_BACKTRACE=1 target/$MODE/dpdkIO -f sgx-runner/config.toml
fi


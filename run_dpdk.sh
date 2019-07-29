#!/bin/bash
source ./config.sh
set -e

TASK=macswap

BASE_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd)"

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

NATIVE_LIB_PATH="${BASE_DIR}/native"

# Execute
export PATH="${BIN_DIR}:${PATH}"
export LD_LIBRARY_PATH="${NATIVE_LIB_PATH}:${DPDK_LD_PATH}:${LD_LIBRARY_PATH}"
# echo "sudo env PATH=\"$PATH\" LD_LIBRARY_PATH=\"$LD_LIBRARY_PATH\" LD_PRELOAD=\"$LD_PRELOAD\" $executable \"$@\""

if [ $# -eq 1 ]; then
    env PATH="$PATH" LD_LIBRARY_PATH="$LD_LIBRARY_PATH" LD_PRELOAD="$LD_PRELOAD" \
    RUST_BACKTRACE=1 target/$MODE/dpdkIO -f sgx-runner/config_$1core.toml
else
    env PATH="$PATH" LD_LIBRARY_PATH="$LD_LIBRARY_PATH" LD_PRELOAD="$LD_PRELOAD" \
    RUST_BACKTRACE=1 target/$MODE/dpdkIO -p $PORT_OPTIONS -c 0
fi


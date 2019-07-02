# !/bin/bash

# for both real and local run
HOME=/home/yangz
# HOME=/users/yangzhou

export LD_LIBRARY_PATH="$HOME/NetBricks/native:/opt/dpdk/dpdk-stable-17.08/build/lib:"

# for real run
PORT=0000:04:00.0
CORE=0
POOL_SIZE=512
MODE=release
TIME=300
#!/bin/bash

PORT=0000:06:00.0
CORE=0
POOL_SIZE=2048

export LD_LIBRARY_PATH="/home/yangz/NetBricks/native:/opt/dpdk/dpdk-stable-17.08/build/lib:"

TASK=macswap

if [ $# == 1 ]; then
    TASK=$1
fi

echo $TASK

heaptrack /home/yangz/NetBricks/target/release/$TASK \
-p $PORT -c $CORE --pool-size=$POOL_SIZE
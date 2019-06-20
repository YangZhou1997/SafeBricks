#!/bin/bash

PORT=0000:81:00.1
CORE=0
POOL_SIZE=512
MODE=debug
HOME=/users/yangzhou

export LD_LIBRARY_PATH="$HOME/NetBricks/native:/opt/dpdk/dpdk-stable-17.08/build/lib:"

TASK=macswap

if [ $# == 1 ]; then
    TASK=$1
fi

echo $TASK

$HOME/NetBricks/target/$MODE/$TASK \
-p $PORT -c $CORE --pool-size=$POOL_SIZE
#!/bin/bash
source ./config.sh

TASK=macswap

if [ $# == 1 ]; then
    TASK=$1
fi

echo $TASK

$HOME/NetBricks/target/$MODE/$TASK \
-p $PORT -c $CORE --pool-size=$POOL_SIZE
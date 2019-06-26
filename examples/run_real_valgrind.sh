#!/bin/bash
source ./config.sh

# Seems valgrind cannot run in release binary.

TASK=macswap

if [ $# == 1 ]; then
    TASK=$1
fi

echo $TASK

valgrind --tool=massif $HOME/NetBricks/target/$MODE/$TASK \
-p $PORT -c $CORE --pool-size=$POOL_SIZE -d $TIME

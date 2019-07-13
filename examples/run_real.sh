#!/bin/bash
source ./config.sh

TASK=macswap

if [ $# -ge 1 ]; then
    TASK=$1
fi

echo $TASK

if [ $# == 2 ]; then
    $HOME/SafeBricks/target/$MODE/$TASK --secondary
else
    $HOME/SafeBricks/target/$MODE/$TASK\
    -p $PORT -c $CORE --pool-size=$POOL_SIZE
fi

unset RUST_BACKTRACE

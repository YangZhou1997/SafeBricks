# !/bin/bash
source ./config.sh

TASK=macswap

if [ $# == 1 ]; then
    TASK=$1
fi

echo $TASK

# env LD_PRELOAD=$HOME/jemalloc/lib/libjemalloc.so $HOME/NetBricks/target/$MODE/$TASK \
# -p $PORT -c $CORE --pool-size=$POOL_SIZE -d $TIME \
# 2>&1 | grep Tracing --line-buffered | awk '{$3=$3/(1024.0)} {print}'

env LD_PRELOAD=$HOME/jemalloc/lib/libjemalloc.so $HOME/NetBricks/target/$MODE/$TASK \
-p $PORT -c $CORE --pool-size=$POOL_SIZE -d $TIME
# \
# 2>&1 | grep Tracing --line-buffered | awk '{$3=$3/(1024.0)} {print}'


# > jemalloc.log
# awk '{$3=$3/(1024.0)} {print}' jemalloc.log > jemalloc.log


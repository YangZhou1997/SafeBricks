# !/bin/bash
source ./config.sh

TASK=macswap

if [ $# == 1 ]; then
    TASK=$1
fi

echo $TASK

env LD_PRELOAD=$HOME/jemalloc/lib/libjemalloc.so $HOME/NetBricks/target/$MODE/$TASK \
-p dpdk:eth_pcap0,rx_pcap=$TRAFFIC,tx_pcap=/tmp/out.pcap -c $CORE --pool-size=$POOL_SIZE -d $TIME \
2>&1 | grep Tracing --line-buffered > jemalloc-log/container_1g/$TASK.log


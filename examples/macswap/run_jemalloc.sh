# !/bin/bash

export LD_LIBRARY_PATH="/home/yangz/NetBricks/native:/opt/dpdk/dpdk-stable-17.08/build/lib:"

env LD_PRELOAD=/home/yangz/jemalloc/lib/libjemalloc.so /home/yangz/NetBricks/target/debug/macswap \
-p dpdk:eth_pcap0,rx_pcap=data/http_lemmy.pcap,tx_pcap=/tmp/out.pcap -c 1 -d 1 \
2>&1 | grep Tracing | tee heap.log
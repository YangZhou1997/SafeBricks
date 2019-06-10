#!/bin/bash

export LD_LIBRARY_PATH="/home/yangz/NetBricks/native:/opt/dpdk/dpdk-stable-17.08/build/lib:"

valgrind --tool=massif /home/yangz/NetBricks/target/debug/macswap \
-p dpdk:eth_pcap0,rx_pcap=data/http_lemmy.pcap,tx_pcap=/tmp/out.pcap -c 1 -d 1
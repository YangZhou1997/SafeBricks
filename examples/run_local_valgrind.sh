#!/bin/bash

export LD_LIBRARY_PATH="/home/yangz/NetBricks/native:/opt/dpdk/dpdk-stable-17.08/build/lib:"

TASK=macswap

if [ $# == 1 ]; then
    TASK=$1
fi

echo $TASK

valgrind --tool=massif /home/yangz/NetBricks/target/debug/$TASK \
-p dpdk:eth_pcap0,rx_pcap=/home/yangz/traffic/equinix-chicago.dirA.20160121-130000.UTC.anon.pcap,tx_pcap=/tmp/out.pcap -c 1 -d 1

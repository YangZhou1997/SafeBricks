# !/bin/bash

# for both real and local run
#HOME=/home/vagrant
# HOME=/users/yangzhou
HOME=/home/yangz
# HOME=/opt

TRAFFIC=$HOME/traffic/ictf2010_trim/merged.pcap
# TRAFFIC=$HOME/traffic/ictf2010/merged.pcap


export LD_LIBRARY_PATH="$HOME/NetBricks/native:/opt/dpdk/dpdk-stable-17.08/build/lib:"
export RUST_BACKTRACE=1

# for real run
PORT=0000:02:00.0
CORE=0
POOL_SIZE=512
MODE=release
TIME=1800

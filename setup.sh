#!/bin/bash

sudo bash ../utils/vm-kernel-upgrade.sh
sudo bash ../utils/vm-setup.sh

DPDK_HOME=/users/yangzhou/tools/dpdk-stable-17.08.1
CFLAGS=-g3 -Wno-error=maybe-uninitialized -fPIC

curl -sSf https://fast.dpdk.org/rel/dpdk-17.08.1.tar.xz | tar -xJv

cd $DPDK_HOME

make config T=x86_64-native-linuxapp-gcc EXTRA_CFLAGS="${CFLAGS}"
make -j8 EXTRA_CFLAGS="${CFLAGS}"
sudo make install

sudo insmod $DPDK_HOME/build/kmod/igb_uio.ko

sudo $DPDK_HOME/usertools/dpdk-devbind.py --force -b 0000:81:00.1 igb_uio



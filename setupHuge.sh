#!/bin/bash

sudo chown -R yangzhou:lambda-mpi-PG0 ~/NetBricks
sudo chown -R yangzhou:lambda-mpi-PG0 /dev/hugepages/

echo 2048 | sudo tee /sys/devices/system/node/node0/hugepages/hugepages-2048kB/nr_hugepages
echo 2048 | sudo tee /sys/devices/system/node/node1/hugepages/hugepages-2048kB/nr_hugepages

# check hugepages
cat /proc/meminfo | grep Huge

#release hugepages - way 1
sudo rm -rf /dev/hugepages/*

# release hugepages - way 2
sudo lsof | grep '/dev/hugepages'
sudo kill -9 xxxxx

#!/bin/bash

#sudo chown -R yangzhou:lambda-mpi-PG0 ~/NetBricks

curl https://sh.rustup.rs -sSf | sh  # Install rustup
source $HOME/.cargo/env
rustup install nightly
rustup default nightly

#dependencies for netbricks
sudo apt-get -y install clang libclang-dev libsctp-dev libssl-dev cmake

# hugepages setup on numa node
echo 1024 | sudo tee /sys/devices/system/node/node0/hugepages/hugepages-2048kB/nr_hugepages
echo 1024 | sudo tee /sys/devices/system/node/node1/hugepages/hugepages-2048kB/nr_hugepages

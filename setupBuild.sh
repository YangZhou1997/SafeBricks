#!/bin/bash

#sudo chown -R yangzhou:lambda-mpi-PG0 ~/NetBricks

curl https://sh.rustup.rs -sSf | sh  # Install rustup
source $HOME/.cargo/env
rustup install nightly
rustup default nightly

#dependencies for netbricks
sudo apt-get -y install clang libclang-dev libsctp-dev


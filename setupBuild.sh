#!/bin/bash

sudo chown -R yangzhou:lambda-mpi-PG0 /users/yangzhou/NetBricks

https://sh.rustup.rs -sSf | sh  # Install rustup
source $HOME/.cargo/env
rustup install nightly
rustup default nightly

#dependencies for netbricks
sudo apt-get install clang libclang-dev libclang-dev


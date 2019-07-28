#!/bin/bash

CFLAGS="-g3 -Wno-error=maybe-uninitialized -fPIC"
EXTRA_CFLAGS="${CFLAGS}"
gcc -o mapping mapping.c ${CFLAGS}
./mapping
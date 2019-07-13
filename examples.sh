#!/bin/bash
# Lists all the examples that are runnable and/or testable.
# This is used by the build script.
export examples=(
        # examples/echo-reply
        # examples/ipv4or6
        # examples/mtu-too-big
        # non-runnable examples
        # examples/op-errors
        # examples/signals
        ### Runnable examples | No Tests associated
        ### =======================================
        # examples/ttl-chain
        # examples/collect-metrics
        # examples/embedded-scheduler
        # examples/embedded-scheduler-dependency
        # examples/sctp
        # 
        # examples/tcp-reconstruction
        ### NFs for experiments
        # examples/macswap
        # examples/acl-fw
        # examples/lpm
        # examples/nat-tcp-v4
        # examples/maglev
        # examples/dpi
        # examples/monitoring
        # examples/macswap-ipsec
        # examples/acl-fw-ipsec
        # examples/lpm-ipsec
        # examples/nat-tcp-v4-ipsec
        # examples/maglev-ipsec
        # examples/dpi-ipsec
        # examples/monitoring-ipsec
)

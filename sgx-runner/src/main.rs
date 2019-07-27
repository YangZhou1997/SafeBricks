/* Copyright (c) Fortanix, Inc.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
#[macro_use]
extern crate lazy_static;
extern crate netbricks;
extern crate pktpuller;
extern crate ctrlc;
pub mod haproxy;

use pktpuller::common::Result as PktResult;
use pktpuller::config::load_config;
use pktpuller::interface::{PmdPort, PacketRx, PacketTx, PortQueue};
use pktpuller::operators::{Batch, ReceiveBatch};
use pktpuller::operators::BATCH_SIZE;
use pktpuller::packets::{Ethernet, Packet, RawPacket};
use pktpuller::runtime::Runtime;
use pktpuller::scheduler::Executable;
use pktpuller::heap_ring::ring_buffer::*;
use pktpuller::native::mbuf::MBuf;
use pktpuller::config::{NUM_RXD, NUM_TXD};
use pktpuller::allocators::CacheAligned;

use haproxy::{run_client, run_server, parse_args};

use netbricks::heap_ring::ring_buffer::RingBuffer as RingBufferSGX;
use netbricks::native::mbuf::MBuf as MBufSGX;

use std::thread;
use std::sync::{Arc, Mutex};
use std::fmt::Display;

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering, compiler_fence};

const PKT_NUM: u64 = (8 * 1024 * 1024);
const PRINT_INTER: u64 = (1024 * 1024);

// pull_count;
lazy_static!{
    static ref BATCH_CNT: Mutex<Vec<u64>> = {
        let batch_cnt = (0..1).map(|_| 0 as u64).collect();        
        Mutex::new(batch_cnt)
    };
}

// pull_count;
lazy_static!{
    static ref BATCH_CNT_SGX: Mutex<Vec<u64>> = {
        let batch_cnt = (0..1).map(|_| 0 as u64).collect();        
        Mutex::new(batch_cnt)
    };
}

// TODO: extract the config and ring_buffer from the pkupuller (separated from dpdk logic)
// Reason: system will crash if dpdk crate co-exists with second enclave creation. 
fn main() -> PktResult<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let configuration = load_config()?;
    println!("{}", configuration);
    let mut runtime = Runtime::init(&configuration)?;

    let main_port_name = &configuration.ports[0].name; // get this hostio core's port name
    let main_port = runtime.context.ports.get(main_port_name).unwrap().clone();

    // we can reuse the ports/queues get from the only port as multiple rings connecting to different enclaves. 
    let mut ports: Vec<CacheAligned<PortQueue>> = Vec::new();
    //  = runtime.context.rx_queues.get(&0).unwrap().clone(); // get this hostio core's queues.
    for (core_id, queue_vec) in runtime.context.rx_queues.iter() {
        ports.extend(queue_vec.iter().cloned());
    }

    let core_ids = core_affinity::get_core_ids().unwrap();
    println!("core_affinity detect: # available cores: {}", core_ids.len());
    assert!(core_ids.len() >= ports.len() + 1 + 1, "# available cores is not enough"); 
    // one core for pktpuller, one core for normal linux monitoring.
    core_affinity::set_for_current(core_ids[1].clone());

    let mut server_count: u64 = 0;
    let mut client_count: u64 = 0;
    let file = parse_args().unwrap();

    let mut recvq_ring: Vec<RingBuffer> = Vec::new();
    let mut sendq_ring: Vec<RingBuffer> = Vec::new();

    println!("ports number: {}", ports.len());


    for (i, queue) in ports.iter().enumerate() {
        // Create two shared queue: recvq and sendq; 
        recvq_ring.push(unsafe{RingBuffer::new_in_heap((NUM_RXD) as usize, &format!("{}_{}", RECVQ_PREFIX, i))}.unwrap());
        sendq_ring.push(unsafe{RingBuffer::new_in_heap((NUM_TXD) as usize, &format!("{}_{}", SENDQ_PREFIX, i))}.unwrap());

        let core_ids_sgx = core_ids[i + 2].clone();
        let file_core = file.clone();
        let server = thread::spawn(move || {
            core_affinity::set_for_current(core_ids_sgx);
            run_server(file_core).unwrap();
            // server_count += run_server_thread().unwrap();
        });

        let recvq_addr_u64: u64 = recvq_ring[i].head.my_usize as u64; // *mut usize
        let sendq_addr_u64: u64 = sendq_ring[i].head.my_usize as u64;

        println!("recvq_addr {}, sendq_addr {}", recvq_addr_u64, sendq_addr_u64);
        // send recvq_addr and sendq_addr to the enclave through TCP tunnel. 
        run_client(recvq_addr_u64, sendq_addr_u64).unwrap(); // recvq_addr, sendq_addr

        println!("out-of-enclave: {}, {}, {}, {}", recvq_ring[i].head(), recvq_ring[i].tail(), recvq_ring[i].size(), recvq_ring[i].mask());
        println!("out-of-enclave: {}, {}, {}, {}", recvq_ring[i].head(), recvq_ring[i].tail(), recvq_ring[i].size(), recvq_ring[i].mask());
        
        thread::sleep(std::time::Duration::from_secs(2));// wait until server in enclave sets up;
    }

    println!("{} vs. {}", client_count, server_count);
    
    // directly exit and let enclaves run.
    Ok(())
}

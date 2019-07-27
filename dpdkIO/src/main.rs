/* Copyright (c) Fortanix, Inc.
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
#[macro_use]
extern crate lazy_static;
extern crate pktpuller;
extern crate ctrlc;
extern crate sharedring;

use pktpuller::common::Result as PktResult;
use pktpuller::config::load_config;
use pktpuller::interface::{PmdPort, PacketRx, PacketTx, PortQueue};
use pktpuller::operators::{Batch, ReceiveBatch};
use pktpuller::operators::BATCH_SIZE;
use pktpuller::packets::{Ethernet, Packet, RawPacket};
use pktpuller::runtime::Runtime;
use pktpuller::scheduler::Executable;
use pktpuller::native::mbuf::MBuf;
use pktpuller::config::{NUM_RXD, NUM_TXD};
use pktpuller::allocators::CacheAligned;

use sharedring::ring_buffer::*;

use std::thread;
use std::sync::{Arc, Mutex};
use std::fmt::Display;

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering, compiler_fence};

use std::io::stdout;
use std::io::Write;

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


#[derive(Clone)]
struct MbufVec { my_mbufs: Vec::<*mut MBuf> }

impl Drop for MbufVec {
    fn drop(&mut self) {
        unsafe {
            println!("MbufVec outside freed");
        }
    }
}

// This "ports" is essentially "queues"
fn hostio<T, >(main_port: Arc<PmdPort>, ports: Vec<T>, recvq_ring: &mut Vec<RingBuffer>, sendq_ring: &mut Vec<RingBuffer>, running: Arc<AtomicBool>) -> std::io::Result<u64>
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
{
    for port in &ports {
        println!("Receiving port {}", port);
    }

    let mut mbufs = MbufVec{ my_mbufs: Vec::<*mut MBuf>::with_capacity(BATCH_SIZE) };
    let mut pull_count: [u64; 4] = [0u64; 4];
    let mut pkt_count_from_nic: [u64; 4] = [0u64; 4];
    let mut pkt_count_from_enclave: [u64; 4] = [0u64; 4];

    while running.load(Ordering::SeqCst) {
        for (index, queue) in ports.iter().enumerate() {
            let i = index as usize;
            // hostio only used ports[0];
            unsafe{ mbufs.my_mbufs.set_len(BATCH_SIZE) }; 

            let mut recv_pkt_num_from_nic: u32 = 0;
            // if pkt_count_from_nic < PKT_NUM {
                // pull packets from NIC; write mbuf pointers to mbufs.     
                recv_pkt_num_from_nic = match ports[i].recv(mbufs.my_mbufs.as_mut_slice()) {
                    Ok(received) => {
                        unsafe{ mbufs.my_mbufs.set_len(received as usize) };
                        received
                    }
                    // the underlying DPDK method `rte_eth_rx_burst` will
                    // never return an error. The error arm is unreachable
                    _ => unreachable!(),
                };
                unsafe{ mbufs.my_mbufs.set_len(recv_pkt_num_from_nic as usize) }; 
            // }
            // else {
            //     unsafe{ mbufs.my_mbufs.set_len(0) }; 
            // }

            
            // push recv_pkt_num_from_nic mbuf pointers to recvq.      
            if !mbufs.my_mbufs.is_empty() {
                let mut to_send = mbufs.my_mbufs.len();
                while to_send > 0 && running.load(Ordering::SeqCst) {
                    let sent = unsafe{ recvq_ring[i].write_at_tail(std::mem::transmute::<&[*mut MBuf], &[u64]>(mbufs.my_mbufs.as_slice())) };
                    to_send -= sent;
                    if to_send > 0 {
                        mbufs.my_mbufs.drain(..sent);
                    }
                }
                unsafe {
                    unsafe{ mbufs.my_mbufs.set_len(0) };
                }
            }
            
            // thread::sleep(std::time::Duration::from_secs(1));// for debugging;

            // hostio only used ports[0];
            unsafe{ mbufs.my_mbufs.set_len(BATCH_SIZE) }; 

            // pull packet from sendq;
            let recv_pkt_num_from_enclave = unsafe{ sendq_ring[i].read_from_head(std::mem::transmute::<&mut [*mut MBuf], &mut [u64]>(mbufs.my_mbufs.as_mut_slice())) };
            unsafe{ mbufs.my_mbufs.set_len(recv_pkt_num_from_enclave) }; 

            // Send pacekt to dpdk port;
            if !mbufs.my_mbufs.is_empty() {
                let mut to_send = mbufs.my_mbufs.len();
                while to_send > 0 && running.load(Ordering::SeqCst){
                    match ports[i].send(mbufs.my_mbufs.as_mut_slice()) {
                        Ok(sent) => {
                            let sent = sent as usize;
                            to_send -= sent;
                            if to_send > 0 {
                                mbufs.my_mbufs.drain(..sent);
                            }
                        }
                        // the underlying DPDK method `rte_eth_tx_burst` will
                        // never return an error. The error arm is unreachable
                        _ => unreachable!(),
                    }
                }
                unsafe {
                    unsafe{ mbufs.my_mbufs.set_len(0) };
                }
            }
            pkt_count_from_nic[i] += recv_pkt_num_from_nic as u64;
            pkt_count_from_enclave[i] += recv_pkt_num_from_enclave as u64;

            pull_count[i] += 1;

            // if pkt_count_from_enclave[i] % PRINT_INTER == 0 {
                // if pkt_count_from_enclave[i] != 0 && recv_pkt_num_from_enclave != 0 {
                    let (rx, tx) = main_port.stats(0);
                    println!("Ring {} out-of-enclave: from nic {}, to sgx {}, from sgx {}, to nic {}", i, rx, pkt_count_from_nic[i], pkt_count_from_enclave[i], tx);
                    println!("  recvq: head {} vs. tail {}", recvq_ring[i].head(), recvq_ring[i].tail());
                    println!("  sendq: head {} vs. tail {}", sendq_ring[i].head(), sendq_ring[i].tail());
                // }
            // }
        thread::sleep(std::time::Duration::from_secs(1));

            // if pkt_count_from_nic >= PKT_NUM && pkt_count_from_enclave >= PKT_NUM {
            //     break;
            // }
        }
    }
    println!("exit from loop");
    // either not break above or have a loop here. 
    for (i, queue) in ports.iter().enumerate() {
        compiler_fence(Ordering::Release);
        recvq_ring[i as usize].set_size(STOP_MARK as usize);
    }
    
    Ok(pkt_count_from_nic.iter().sum())
}


fn main() -> PktResult<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("singnal received");
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
    core_affinity::set_for_current(core_ids[0].clone());

    let mut server_count: u64 = 0;
    let mut client_count: u64 = 0;

    let mut recvq_ring: Vec<RingBuffer> = Vec::new();
    let mut sendq_ring: Vec<RingBuffer> = Vec::new();

    println!("ports number: {}", ports.len());

    for (i, queue) in ports.iter().enumerate() {
        // Create two shared queue: recvq and sendq in shared memory with name.
        recvq_ring.push(unsafe{RingBuffer::new_in_heap(NUM_RXD as usize, &format!("{}_{}", RECVQ_PREFIX, i))}.unwrap());
        sendq_ring.push(unsafe{RingBuffer::new_in_heap(NUM_TXD as usize, &format!("{}_{}", SENDQ_PREFIX, i))}.unwrap());

        let recvq_addr_u64: u64 = recvq_ring[i].head.my_usize as u64; // *mut usize
        let sendq_addr_u64: u64 = sendq_ring[i].head.my_usize as u64;

        println!("recvq_addr {}, sendq_addr {}", recvq_addr_u64, sendq_addr_u64);

        println!("out-of-enclave: {}, {}, {}, {}", recvq_ring[i].head(), recvq_ring[i].tail(), recvq_ring[i].size(), recvq_ring[i].mask());
        println!("out-of-enclave: {}, {}, {}, {}", recvq_ring[i].head(), recvq_ring[i].tail(), recvq_ring[i].size(), recvq_ring[i].mask());
    }
    println!("to host io");

    // keep pulling packet from DPDK port, and push pkt pointers to recvq
    // keep pulling packet pointers from sendq, and send them out to the DPDK port.
    client_count = hostio(main_port, ports, &mut recvq_ring, &mut sendq_ring, running).unwrap();

    println!("{} vs. {}", client_count, server_count);
    Ok(())
}

#[macro_use]
extern crate lazy_static;
extern crate netbricks;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::{slice};
use netbricks::config::{NUM_RXD, NUM_TXD};
use netbricks::operators::BATCH_SIZE;
use netbricks::heap_ring::ring_buffer::RingBuffer as RingBufferSGX;
use netbricks::native::mbuf::MBuf as MBufSGX;
use netbricks::packets::{Ethernet as EthernetSGX, Packet as PacketSGX, RawPacket as RawPacketSGX};

use std::sync::{Arc, Mutex};

const PKT_NUM: u64 = (1024 * 1024);

// poll_count;
lazy_static!{
    static ref BATCH_CNT_SGX: Mutex<Vec<u64>> = {
        let batch_cnt = (0..1).map(|_| 0 as u64).collect();        
        Mutex::new(batch_cnt)
    };
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("localhost:6010")?;
    let (stream, peer_addr) = listener.accept()?;
    let peer_addr = peer_addr.to_string();
    let local_addr = stream.local_addr()?;
    eprintln!(
        "App:: accept  - local address is {}, peer address is {}",
        local_addr, peer_addr
    );

    let mut reader = BufReader::new(stream);
    let mut message = String::new();
    
    let read_bytes = reader.read_line(&mut message)?;
    print!("{}", message);
    let queue_addr: Vec<u64> = 
            message.trim().split(' ')
        .map(|s| s.parse().unwrap())
        .collect();
    println!("{:?}", queue_addr);

    let recvq_ring = unsafe{ RingBufferSGX::attach_in_heap((NUM_RXD) as usize, queue_addr[0]).unwrap() };
    let sendq_ring = unsafe{ RingBufferSGX::attach_in_heap((NUM_RXD) as usize, queue_addr[1]).unwrap() };
    
    println!("in-enclave: {}, {}, {}, {}", recvq_ring.head(), recvq_ring.tail(), recvq_ring.size(), recvq_ring.mask());    
    // recvq_ring.set_head(56781234);
    // recvq_ring.set_tail(43218765);
    println!("in-enclave: {}, {}, {}, {}", recvq_ring.head(), recvq_ring.tail(), recvq_ring.size(), recvq_ring.mask());    

    let mut mbufs = Vec::<*mut MBufSGX>::with_capacity(BATCH_SIZE);
    let mut poll_count: u64 = 0;
    let mut pkt_count: u64 = 0;
    let mut pull_none: u64 = 0;
    loop{
        // fib(300);
        unsafe{ mbufs.set_len(BATCH_SIZE) };
        let len = mbufs.len() as i32;
        // pull packet from recvq;
        let recv_pkt_num_from_outside = recvq_ring.read_from_head(mbufs.as_mut_slice());
        unsafe{ mbufs.set_len(recv_pkt_num_from_outside) }; 
        
        // let _: Vec<()> = mbufs.iter().map({
        //     |m| {
        //         let mut raw = RawPacket::from_mbuf(*m);
        //         let mut ethernet = raw.parse::<Ethernet>().unwrap();
        //         println!("src: {:?}", ethernet.src());
        //         println!("dst: {:?}", ethernet.dst());
        //         ethernet.swap_addresses();
        //     }
        // }).collect();

        // println!("{}, {}, {}", recv_pkt_num_from_outside, recvq_ring.head(), recvq_ring.tail());

        // let rand_v: f64 = rand::thread_rng().gen();
        // if rand_v < 0.00001 {}
            // poll_count += 1;
            // if poll_count % (1024 * 32) == 0 {
            //     if recv_pkt_num_from_outside > 0 {
            //         pkt_count += recv_pkt_num_from_outside as u64;
            //         let mut raw = RawPacketSGX::from_mbuf(mbufs[0]);
            //         let mut ethernet = raw.parse::<EthernetSGX>().unwrap();
            //         println!("src: {:?}", ethernet.src());
            //         println!("dst: {:?}", ethernet.dst());
            //         ethernet.swap_addresses();
            //         // let _: Vec<()> = mbufs.iter().map({
            //         //     |m| {
            //         //         let mut raw = RawPacket::from_mbuf(*m);
            //         //         let mut ethernet = raw.parse::<Ethernet>().unwrap();
            //         //         println!("src: {:?}", ethernet.src());
            //         //         println!("dst: {:?}", ethernet.dst());
            //         //         ethernet.swap_addresses();
            //         //     }
            //         // }).collect();
            //     }
            // }
        // }


        if !mbufs.is_empty() {
            let mut to_send = mbufs.len();
            while to_send > 0 {
                let sent = sendq_ring.write_at_tail(mbufs.as_mut_slice());
                to_send -= sent;
                if to_send > 0 {
                    mbufs.drain(..sent);
                }
            }
            unsafe {
                unsafe{ mbufs.set_len(0) };
            }
        }
        if recv_pkt_num_from_outside == 0 {
            pull_none += 1;
        }
        else {
            pull_none = 0;
            pkt_count += recv_pkt_num_from_outside as u64;
        }
        // if pkt_count != 0 && pull_none == 0 {
        //     println!("pkt_count: {}", pkt_count);
        // }
        
        // you cannot break, since some memory segmentfault or heap double free error would appear.
        if pkt_count >= PKT_NUM {
             break;
        }
    }
    Ok(())
}

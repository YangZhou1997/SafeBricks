extern crate netbricks;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::{slice};
use netbricks::heap_ring::ring_buffer::*;
use netbricks::config::{NUM_RXD, NUM_TXD};
use netbricks::operators::BATCH_SIZE;
use netbricks::native::mbuf::MBuf;

fn fib(n: u64) -> u64{
    if n == 0{
        return 0;
    }
    else if n == 1{
        return 1;
    }
    else{
        return fib(n - 1) + fib(n - 2); 
    }
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

    let recvq_ring = unsafe{ RingBuffer::attach_in_heap((NUM_RXD * 8) as usize, queue_addr[0]).unwrap() };
    let sendq_ring = unsafe{ RingBuffer::attach_in_heap((NUM_RXD * 8) as usize, queue_addr[1]).unwrap() };
    
    let mut mbufs = Vec::<*mut MBuf>::with_capacity(BATCH_SIZE);
    
    
    loop{
        fib(300);
        println!("loop0");
        unsafe{ mbufs.set_len(BATCH_SIZE) };
        let len = mbufs.len() as i32;
        // pull packet from recvq;
        let b_u8_p_mut = unsafe{ (&mut (*(mbufs[0])) as *mut MBuf) as *mut u8 };
        let b_u8_array_mut = unsafe{ slice::from_raw_parts_mut(b_u8_p_mut, BATCH_SIZE * 8) };
        let recv_pkt_num_from_enclave = recvq_ring.read_from_head(b_u8_array_mut) / 8;
        unsafe{ mbufs.set_len(recv_pkt_num_from_enclave) }; 
        // thread::sleep(std::time::Duration::from_secs(1));// for debugging;
        println!("loop1");
        if !mbufs.is_empty() {
            let mut to_send = mbufs.len();
            while to_send > 0 {
                let b_u8_p = unsafe{ (&(*(mbufs[0])) as *const MBuf) as *const u8 };
                let b_u8_array = unsafe{ slice::from_raw_parts(b_u8_p, to_send * 8) };
                let sent = sendq_ring.write_at_tail(b_u8_array) / 8;
                println!("{}, {}", sent, sendq_ring.tail());
                // thread::sleep(std::time::Duration::from_secs(1));// for debugging;
                to_send -= sent;
                if to_send > 0 {
                    mbufs.drain(..sent);
                }
            }
            unsafe {
                unsafe{ mbufs.set_len(0) };
            }
        }
    }
        

    Ok(())
}

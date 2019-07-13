extern crate netbricks;
use netbricks::common::Result;
// use netbricks::config::load_config;
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::packets::{Ethernet, Packet, RawPacket};
use std::fmt::Display;

fn install(){
    let port: u32 = 0; // the shared memory buffer that NF read/write packet from/to. 
    ReceiveBatch::new(port)
                .map(macswap)
                .send(port);
}

fn macswap(packet: RawPacket) -> Result<Ethernet> {
    assert!(packet.refcnt() == 1);
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    Ok(ethernet)
}

fn main() -> Result<()> {
    // let configuration = load_config()?;
    // println!("{}", configuration);
    // let mut runtime = Runtime::init(&configuration)?;
    install();
    Ok(());
}

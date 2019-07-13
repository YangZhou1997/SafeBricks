extern crate netbricks;
use netbricks::common::Result;
// use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::packets::{Ethernet, Packet, RawPacket};
use std::fmt::Display;


fn install<T, S>(ports: Vec<T>)
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
{
    // the shared memory ring that NF read/write packet from/to.     
    while true {
        let _: Vec<_> = ports
            .iter()
            .map(|port| {
                ReceiveBatch::new(port.clone())
                    .map(macswap)
                    .send(port.clone()).execute()
            })
            .collect();
    }
}

fn macswap(packet: RawPacket) -> Result<Ethernet> {
    // assert!(packet.refcnt() == 1);
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    Ok(ethernet)
}

fn main() -> Result<()> {
    // let configuration = load_config()?;
    // println!("{}", configuration);
    // let mut runtime = Runtime::init(&configuration)?;
    let ports = get_ports(); // somehow get a vector of ports
    install();
    Ok(());
}

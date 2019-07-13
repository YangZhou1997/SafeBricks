extern crate netbricks;
use netbricks::common::Result;
// use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::interface::{SimulateQueue, SimulatePort};
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::packets::{Ethernet, Packet, RawPacket};
use std::fmt::Display;
use std::sync::Arc;

// This "ports" is essentially "queues"
fn install<T, S>(ports: Vec<T>)
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
{
    for port in &ports {
        println!("Receiving port {}", port);
    }

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
    let sim_port = Arc::try_unwrap((SimulatePort::new(1)).unwrap()).unwrap(); // somehow get a vector of ports
    let sim_queue = sim_port.new_simulate_queue(1).unwrap();
    let ports = Vec![sim_queue];
    install(ports);
    Ok(());
}

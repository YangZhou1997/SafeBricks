extern crate netbricks;
use netbricks::common::Result;
// use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::interface::{SimulatePort, SimulateQueue};
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::packets::{Ethernet, Packet, RawPacket};
use netbricks::scheduler::Executable;
use std::fmt::Display;
use std::sync::Arc;

use std::io::stdout;
use std::io::Write;

// This "ports" is essentially "queues"
fn install<T, >(main_port: SimulatePort, ports: Vec<T>)
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
{
    for port in &ports {
        println!("Receiving port {}", port);
    }

    // the shared memory ring that NF read/write packet from/to.     
    loop {
        let _: Vec<_> = ports
            .iter()
            .map(|port| {
                ReceiveBatch::new(port.clone())
                    .map(macswap)
                    .send(port.clone()).execute()
            })
            .collect();
        let (rx, tx) = main_port.stats();
        // println!("{} vs. {}", rx, tx);
    }
}

fn macswap(packet: RawPacket) -> Result<Ethernet> {
    // assert!(packet.refcnt() == 1);
    // println!("macswap"); stdout().flush().unwrap();
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    Ok(ethernet)
}

fn main() -> Result<()> {
    // let configuration = load_config()?;
    // println!("{}", configuration);
    // let mut runtime = Runtime::init(&configuration)?;
    println!("hello world"); stdout().flush().unwrap();
    let sim_port = Arc::try_unwrap((SimulatePort::new(1)).unwrap()).unwrap(); // somehow get a vector of ports
    let sim_queue = sim_port.new_simulate_queue(1).unwrap();
    let ports = vec![sim_queue];
    install(sim_port, ports);
    Ok(())
}

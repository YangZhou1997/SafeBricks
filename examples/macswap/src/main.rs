extern crate netbricks;
use netbricks::common::Result;
use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::packets::{Ethernet, Packet, RawPacket};
use netbricks::scheduler::Scheduler;
use netbricks::scheduler::{initialize_system, PKT_NUM};
use std::fmt::Display;
// use std::io::stdout;
// use std::io::Write;
use std::sync::Arc;


// This "ports" is essentially "queues"
fn install<T, S>(ports: Vec<T>, sched: &mut S)
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
    S: Scheduler + Sized,
{
    for port in &ports {
        println!("Receiving port {}", port);
    }

    let pipelines: Vec<_> = ports
        .iter()
        .map(|port| {
            ReceiveBatch::new(port.clone())
                .map(macswap)
                .send(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn macswap(packet: RawPacket) -> Result<Ethernet> {
    assert!(packet.refcnt() == 1);
    // println!("macswap");
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    Ok(ethernet)
}

fn main() -> Result<()> {
    let configuration = load_config()?;
    println!("{}", configuration);
    let mut context = initialize_system(&configuration)?;
    context.run(Arc::new(install), PKT_NUM); // will trap in the run() and return after finish
    Ok(())
}

extern crate fnv;
#[macro_use]
extern crate lazy_static;
extern crate netbricks;
use fnv::FnvHasher;
use netbricks::common::Result;
use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::packets::ip::v4::Ipv4;
use netbricks::packets::ip::Flow;
use netbricks::packets::{Ethernet, Packet, RawPacket, Tcp};
use netbricks::runtime::Runtime;
use netbricks::scheduler::Scheduler;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::BuildHasherDefault;
use std::sync::Arc;
use std::sync::RwLock;
// use std::io::stdout;
// use std::io::Write;


type FnvHash = BuildHasherDefault<FnvHasher>;

lazy_static! {
    static ref FLOW_MAP: Arc<RwLock<HashMap<Flow, u64, FnvHash>>> = {
        let m = HashMap::with_hasher(Default::default());
        Arc::new(RwLock::new(m))
    };
}

fn install<T, S>(ports: Vec<T>, sched: &mut S)
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
    S: Scheduler + Sized,
{
    println!("Receiving started");

    let pipelines: Vec<_> = ports
        .iter()
        .map(move |port| {
            ReceiveBatch::new(port.clone())
                .map(|p| monitoring(p))
                .sendall(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn monitoring(packet: RawPacket) -> Result<Tcp<Ipv4>> {
    // print!("-4");stdout().flush();
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let tcp = v4.parse::<Tcp<Ipv4>>()?;
    let flow = tcp.flow();

    let mut flow_map = FLOW_MAP.write().unwrap();
    let count = flow_map.entry(flow).or_insert(0);
    *count += 1;

    Ok(tcp)
}

fn main() -> Result<()> {
    let configuration = load_config()?;
    println!("{}", configuration);
    let mut runtime = Runtime::init(&configuration)?;
    runtime.add_pipeline_to_run(install);
    runtime.execute()
}

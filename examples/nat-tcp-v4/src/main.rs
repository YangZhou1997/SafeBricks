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
use netbricks::packets::ip::ProtocolNumbers;
use netbricks::packets::ip::{Flow, IpPacket};
use netbricks::packets::{Ethernet, Packet, RawPacket, Tcp};
use netbricks::runtime::Runtime;
use netbricks::scheduler::Scheduler;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::BuildHasherDefault;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicU16, Ordering};
// use std::io::stdout;
// use std::io::Write;
use std::cell::RefCell;

// const MIN_PORT: u16 = 1024;
const MAX_PORT: u16 = 65535;

type FnvHash = BuildHasherDefault<FnvHasher>;

thread_local! {
    pub static PORT_MAP: RefCell<HashMap<Flow, Flow, FnvHash>> = {
        let m = HashMap::with_capacity_and_hasher(65536, Default::default());
        RefCell::new(m)
    };
}

thread_local! {
    pub static FLOW_VEC: RefCell<Vec<FlowUsed>> = {
        let m = (0..65536).map(|_| Default::default()).collect();
        RefCell::new(m)
    };
}

lazy_static! {
    static ref NEXT_PORT: AtomicU16 = { AtomicU16::new(1024) };
}

#[derive(Clone, Default)]
struct Unit;

#[derive(Clone, Copy)]
pub struct FlowUsed {
    pub flow: Flow,
    pub time: u64,
    pub used: bool,
}

trait Stamper {
    fn stamp_flow(&mut self, flow: Flow) -> Result<()>;
}

impl<E: IpPacket> Stamper for Tcp<E> {
    fn stamp_flow(&mut self, flow: Flow) -> Result<()> {
        self.envelope_mut().set_src(flow.src_ip())?;
        self.envelope_mut().set_dst(flow.dst_ip())?;
        self.set_src_port(flow.src_port());
        self.set_dst_port(flow.dst_port());
        Ok(())
    }
}

impl Default for FlowUsed {
    fn default() -> FlowUsed {
        FlowUsed {
            flow: Flow::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                0,
                0,
                ProtocolNumbers::Tcp,
            ),
            time: 0,
            used: false,
        }
    }
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
                .map(|p| nat(p, Ipv4Addr::new(10, 0, 0, 1)))
                .sendall(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn nat(packet: RawPacket, nat_ip: Ipv4Addr) -> Result<Tcp<Ipv4>> {
    // print!("-4");stdout().flush();
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let mut tcp = v4.parse::<Tcp<Ipv4>>()?;
    let flow = tcp.flow();

    PORT_MAP.with(|port_map| {
        let port_map_lived = port_map.borrow();
        let exist_res = port_map_lived.get(&flow);
        match exist_res {
            Some(s) => {
                // drop(port_map);
                let _ = tcp.stamp_flow(*s);
                tcp.cascade();
            }
            None => {
                drop(port_map_lived);

                if NEXT_PORT.load(Ordering::Relaxed) < MAX_PORT {
                    let assigned_port = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
                    FLOW_VEC.with(|flow_vec| {
                        let mut flow_vec_lived = flow_vec.borrow_mut();
                        flow_vec_lived[assigned_port as usize].flow = flow;
                        flow_vec_lived[assigned_port as usize].used = true; 
                    });

                    let mut outgoing_flow = flow;
                    outgoing_flow.set_src_ip(IpAddr::V4(nat_ip));
                    outgoing_flow.set_src_port(assigned_port);
                    let rev_flow = outgoing_flow.reverse();
                    
                    PORT_MAP.with(|port_map2|{
                        let mut port_map_mut_lived = port_map2.borrow_mut();
                        port_map_mut_lived.insert(flow, outgoing_flow);
                        port_map_mut_lived.insert(rev_flow, flow.reverse());
                    });
                    let _ = tcp.stamp_flow(outgoing_flow);
                    tcp.cascade();
                }
            }
        }
    });

    Ok(tcp)
}

fn main() -> Result<()> {
    let configuration = load_config()?;
    println!("{}", configuration);
    let mut runtime = Runtime::init(&configuration)?;
    runtime.add_pipeline_to_run(install);
    runtime.execute()
}

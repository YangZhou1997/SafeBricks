extern crate fnv;
#[macro_use]
extern crate lazy_static;
extern crate netbricks;
extern crate twox_hash;
use fnv::FnvHasher;
use netbricks::common::Result;
use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::packets::ip::v4::Ipv4;
use netbricks::packets::ip::{Flow, IpPacket};
use netbricks::packets::{Ethernet, Packet, RawPacket, Tcp};
use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr};
use std::io::stdout;
use std::io::Write;
use std::hash::{BuildHasherDefault, BuildHasher, Hash, Hasher};
use twox_hash::XxHash;
use std::slice;
use std::mem;
// use std::sync::RwLock;
// use std::collections::HashMap;
use std::cell::RefCell;
use netbricks::scheduler::Scheduler;
use netbricks::scheduler::{initialize_system, PKT_NUM};
use std::sync::Arc;


const ENTRY_NUM: u32 = 65537;

type FnvHash = BuildHasherDefault<FnvHasher>;
type XxHashFactory = BuildHasherDefault<XxHash>;

pub struct Maglev {
    // permutation: Box<Vec<Vec<u32>>>,
    lut: Box<Vec<u32>>,
    lut_size: u32,
}

impl Maglev {
    pub fn offset_skip_for_name(
        name: &str,
        h1: &FnvHash,
        h2: &XxHashFactory,
        lsize: u32,
    ) -> (u32, u32) {
        let mut fnv_state = h1.build_hasher();
        name.hash(&mut fnv_state);
        let hash1 = fnv_state.finish() as u32;
        let mut xx_state = h2.build_hasher();
        name.hash(&mut xx_state);
        let hash2 = xx_state.finish() as u32;
        let offset = hash2 % lsize;
        let skip = hash1 % (lsize - 1) + 1;
        (offset, skip)
    }

    pub fn generate_permutations(backends: &[&str], lsize: u32) -> Vec<Vec<u32>> {
        println!("Generating permutations");
        let fnv_hasher: FnvHash = Default::default();
        let xx_hasher: XxHashFactory = Default::default();
        backends
            .iter()
            .map(|n| Maglev::offset_skip_for_name(n, &fnv_hasher, &xx_hasher, lsize))
            .map(|(offset, skip)| (0..lsize).map(|j| (offset + j * skip) % lsize).collect())
            .collect()
    }

    fn generate_lut(permutations: &Vec<Vec<u32>>, size: u32) -> Box<Vec<u32>> {
        let mut next: Vec<_> = permutations.iter().map(|_| 0).collect();
        let mut entry: Box<Vec<u32>> = Box::new((0..size).map(|_| 0x8000).collect());
        let mut n = 0;
        println!("Generating LUT");
        while n < size {
            for i in 0..next.len() {
                let mut c = permutations[i][next[i]];
                while entry[c as usize] != 0x8000 {
                    next[i] += 1;
                    c = permutations[i][next[i]];
                }
                if entry[c as usize] == 0x8000 {
                    entry[c as usize] = i as u32;
                    next[i] += 1;
                    n += 1;
                }
                if n >= size {
                    break;
                }
            }
        }
        println!("Done Generating LUT");
        entry
    }

    pub fn new(name: &[&str], lsize: u32) -> Maglev {
        let permutations = Box::new(Maglev::generate_permutations(name, lsize));
        Maglev {
            lut: Maglev::generate_lut(&*permutations, lsize),
            lut_size: lsize,
        }
    }

    #[inline]
    fn flow_as_u8(flow: &Flow) -> &[u8] {
        let size = mem::size_of::<Flow>();
        unsafe { slice::from_raw_parts((flow as *const Flow) as *const u8, size) }
    }
    
    #[inline]
    fn flow_hash(flow: &Flow) -> u32 {
        let mut hasher = FnvHasher::default();
        hasher.write(Maglev::flow_as_u8(flow));
        hasher.finish() as u32
        // farmhash::hash32(flow_as_u8(flow))
    }

    pub fn lookup(&self, flow: &Flow) -> u32 {
        let idx = Maglev::flow_hash(flow) % self.lut_size;
        self.lut[idx as usize]
    }
}

thread_local! {
    pub static LUT: RefCell<Maglev> = {
        let backends = vec!["Larry", "Curly", "Moe"];
        // let ct = backends.len();
        let lut = Maglev::new(&backends, ENTRY_NUM);
        RefCell::new(lut)
    };
}

// lazy_static! {
//     static ref FLOW_CACHE: Arc<RwLock<HashMap<Flow, u32, FnvHash>>> = {
//         let m = HashMap::with_hasher(Default::default());
//         Arc::new(RwLock::new(m))
//     };
// }

trait Stamper {
    fn stamp_flow(&mut self, dst_ip: u32) -> Result<()>;
}

impl<E: IpPacket> Stamper for Tcp<E> {
    fn stamp_flow(&mut self, dst_ip: u32) -> Result<()> {
        self.envelope_mut().set_dst(IpAddr::V4(Ipv4Addr::new(((dst_ip >> 24) & 0xFF) as u8,
             ((dst_ip >> 16) & 0xFF) as u8, ((dst_ip >> 8) & 0xFF) as u8, (dst_ip & 0xFF) as u8)))?;
        Ok(())
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
        .map(|port| {
            ReceiveBatch::new(port.clone())
                .map(|p| lb(p))
                .sendall(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn lb(packet: RawPacket) -> Result<Tcp<Ipv4>> {
// fn lb(packet: RawPacket) -> Result<Ethernet> {
	let mut ethernet = packet.parse::<Ethernet>()?;
	ethernet.swap_addresses();
	let v4 = ethernet.parse::<Ipv4>()?;
	let mut tcp = v4.parse::<Tcp<Ipv4>>()?;
    let flow = tcp.flow(); // new a Flow structure
    let assigned_server = LUT.with(|lut| {
        lut.borrow().lookup(&flow) as u32
    });
    tcp.stamp_flow(assigned_server).unwrap();
    tcp.cascade();

    // Using a hashmap as "fast" translation as implemented in NetBricks paper; 
    // however, results show it hurts performance. 

    // let flow_cache = FLOW_CACHE.read().unwrap();
    // let exist_res = flow_cache.get(&flow);
    // match exist_res {
    //     Some(s) => {
    //         // drop(port_map);
    //         let assigned_server = *s;
    //         tcp.stamp_flow(assigned_server).unwrap();
    //         tcp.cascade();
    //     }
    //     None => {
    //         drop(flow_cache);
    //         let assigned_server = LUT.lookup(&flow) as u32;
            
    //         tcp.stamp_flow(assigned_server).unwrap();
    //         tcp.cascade();

    //         let mut flow_cache = FLOW_CACHE.write().unwrap();
    //         flow_cache.insert(flow, assigned_server);
    //     }
    // }
    Ok(tcp)
}

fn main() -> Result<()> {
    let configuration = load_config()?;
    println!("{}", configuration);
    let mut context = initialize_system(&configuration)?;
    context.run(Arc::new(install), PKT_NUM); // will trap in the run() and return after finish
    Ok(())
}

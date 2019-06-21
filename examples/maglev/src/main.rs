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
use netbricks::runtime::Runtime;
use netbricks::scheduler::Scheduler;
use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
// use std::io::stdout;
// use std::io::Write;
use std::hash::{BuildHasherDefault, BuildHasher, Hash, Hasher};
use twox_hash::XxHash;
use std::slice;
use std::mem;


const ENTRY_NUM: usize = 65537;

type FnvHash = BuildHasherDefault<FnvHasher>;
type XxHashFactory = BuildHasherDefault<XxHash>;

struct Maglev {
    // permutation: Box<Vec<Vec<usize>>>,
    lut: Box<Vec<usize>>,
    lut_size: usize,
}

impl Maglev {
    pub fn offset_skip_for_name(
        name: &str,
        h1: &FnvHash,
        h2: &XxHashFactory,
        lsize: usize,
    ) -> (usize, usize) {
        let mut fnv_state = h1.build_hasher();
        name.hash(&mut fnv_state);
        let hash1 = fnv_state.finish() as usize;
        let mut xx_state = h2.build_hasher();
        name.hash(&mut xx_state);
        let hash2 = xx_state.finish() as usize;
        let offset = hash2 % lsize;
        let skip = hash1 % (lsize - 1) + 1;
        (offset, skip)
    }

    pub fn generate_permutations(backends: &[&str], lsize: usize) -> Vec<Vec<usize>> {
        println!("Generating permutations");
        let fnv_hasher: FnvHash = Default::default();
        let xx_hasher: XxHashFactory = Default::default();
        backends
            .iter()
            .map(|n| Maglev::offset_skip_for_name(n, &fnv_hasher, &xx_hasher, lsize))
            .map(|(offset, skip)| (0..lsize).map(|j| (offset + j * skip) % lsize).collect())
            .collect()
    }

    fn generate_lut(permutations: &Vec<Vec<usize>>, size: usize) -> Box<Vec<usize>> {
        let mut next: Vec<_> = permutations.iter().map(|_| 0).collect();
        let mut entry: Box<Vec<usize>> = Box::new((0..size).map(|_| 0x8000).collect());
        let mut n = 0;
        println!("Generating LUT");
        while n < size {
            for i in 0..next.len() {
                let mut c = permutations[i][next[i]];
                while entry[c] != 0x8000 {
                    next[i] += 1;
                    c = permutations[i][next[i]];
                }
                if entry[c] == 0x8000 {
                    entry[c] = i;
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

    pub fn new(name: &[&str], lsize: usize) -> Maglev {
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
    fn flow_hash(flow: &Flow) -> usize {
        let mut hasher = FnvHasher::default();
        hasher.write(Maglev::flow_as_u8(flow));
        hasher.finish() as usize
        // farmhash::hash32(flow_as_u8(flow))
    }

    pub fn lookup(&self, flow: &Flow) -> usize {
        let idx = Maglev::flow_hash(flow) % self.lut_size;
        self.lut[idx]
    }
}

lazy_static! {
    static ref LUT: Arc<Maglev> = {
        let backends = vec!["Larry", "Curly", "Moe"];
        // let ct = backends.len();
        let lut = Maglev::new(&backends, ENTRY_NUM);
        Arc::new(lut)
    };
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
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let mut tcp = v4.parse::<Tcp<Ipv4>>()?;
    let mut flow = tcp.flow(); // new a Flow structure

    let assigned_server = LUT.lookup(&flow);
    flow.set_dst_ip(IpAddr::V4(Ipv4Addr::new(((assigned_server >> 24) & 0xFF) as u8,
     ((assigned_server >> 16) & 0xFF) as u8, ((assigned_server >> 8) & 0xFF) as u8,
      (assigned_server & 0xFF) as u8)));

    tcp.stamp_flow(flow).unwrap();
    tcp.cascade();

    Ok(tcp)
}

fn main() -> Result<()> {
    let configuration = load_config()?;
    println!("{}", configuration);
    let mut runtime = Runtime::init(&configuration)?;
    runtime.add_pipeline_to_run(install);
    runtime.execute()
}

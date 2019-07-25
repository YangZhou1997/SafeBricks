extern crate fnv;
#[macro_use]
extern crate lazy_static;
extern crate netbricks;
use fnv::FnvHasher;
use netbricks::allocators::CacheAligned;
use netbricks::common::Result;
use netbricks::config::load_config;
use netbricks::interface::*;
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::packets::ip::v4::Ipv4;
use netbricks::packets::ip::Flow;
use netbricks::packets::{Ethernet, Packet, Tcp, RawPacket};
use netbricks::runtime::Runtime;
use netbricks::scheduler::Scheduler;
use netbricks::utils::cidr::v4::Ipv4Cidr;
use netbricks::utils::cidr::Cidr;
use std::collections::HashSet;
use std::hash::BuildHasherDefault;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::RwLock;
use netbricks::utils::ipsec::*;
use std::cell::RefCell;

type FnvHash = BuildHasherDefault<FnvHasher>;

thread_local! {
    pub static FLOW_CACHE: RefCell<HashSet<Flow, FnvHash>> = {
        let m = HashSet::with_hasher(Default::default());
        RefCell::new(m)  
    };
}

thread_local! {
    pub static ACLS: RefCell<Vec<Acl>> = {
        let acl = vec![Acl {
            // 0 and 32 means exactly match. 
            src_prefix: Some(Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 0).unwrap()),
            dst_prefix: None,
            src_port: None,
            dst_port: None,
            established: None,
            drop: false,
        },
        Acl {
            src_prefix: None,
            dst_prefix: Some(Ipv4Cidr::new(Ipv4Addr::new(0, 0, 0, 0), 0).unwrap()),
            src_port: None,
            dst_port: None,
            established: None,
            drop: false,
        },
        Acl {
            src_prefix: None,
            dst_prefix: None,
            src_port: Some(1338 as u16),
            dst_port: None,
            established: None,
            drop: false,
        },
        Acl {
            src_prefix: None,
            dst_prefix: None,
            src_port: None,
            dst_port: Some(1338 as u16),
            established: None,
            drop: false,
        },
        Acl {
            src_prefix: None,
            dst_prefix: None,
            src_port: None,
            dst_port: None,
            established: Some(true),
            drop: false,
        }];
        RefCell::new(acl)
    };
}

#[derive(Clone)]
pub struct Acl {
    pub src_prefix: Option<Ipv4Cidr>,
    pub dst_prefix: Option<Ipv4Cidr>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub established: Option<bool>,
    // TODO: Related not complete
    pub drop: bool,
}

impl Acl {
    // self.prefix == (self.mask & u32::from_be_bytes(address.octets()))
    fn contains(&self, ip: IpAddr) -> bool {
        if let Some(ref prefix) = self.src_prefix {
            prefix.contains_ip(ip)
        } else {
            true
        }
    }

    fn matches(&self, flow: &Flow) -> bool {
        if self.contains(flow.src_ip())
            && self.contains(flow.dst_ip())
            && (self.src_port.is_none() || flow.src_port() == self.src_port.unwrap())
            && (self.dst_port.is_none() || flow.dst_port() == self.dst_port.unwrap())
        {
            if let Some(established) = self.established {
                let rev_flow = flow.reverse();
                FLOW_CACHE.with(|flow_cache| {
                (flow_cache.borrow().contains(flow)
                    || flow_cache.borrow().contains(&rev_flow))
                    == established
                })    
            } else {
                true
            }
        } else {
            false
        }
    }
}

fn install<S: Scheduler + Sized>(ports: Vec<CacheAligned<PortQueue>>, sched: &mut S) {
    for port in &ports {
        println!(
            "Receiving port {} rxq {} txq {}",
            port.port.mac_address(),
            port.rxq(),
            port.txq()
        );
    }

    let pipelines: Vec<_> = ports
        .iter()
        .map(|port| {
            ReceiveBatch::new(port.clone())
                .filter_map(acl_match)
                .sendall(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn acl_match(packet: RawPacket) -> Result<Option<Ipv4>> {
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let payload: &mut [u8] = v4.get_payload_mut(); // payload.len()

    let esp_hdr: &mut [u8] = &mut [0u8; 8];
    esp_hdr.copy_from_slice(&payload[0..ESP_HEADER_LENGTH]);

    let decrypted_pkt: &mut [u8] = &mut [0u8; 2000];
    let decrypted_pkt_len = aes_cbc_sha256_decrypt(payload, decrypted_pkt, false).unwrap();

    let flow = get_flow(decrypted_pkt);
    let mut match_res: bool = ACLS.with(|acls| {    
        if let Some(acl) = acls.borrow().iter().find(|ref acl| acl.matches(&flow)) {
            if !acl.drop {
                FLOW_CACHE.with(|flow_cache| {
                    (*flow_cache.borrow_mut()).insert(flow);
                });
            }
            true
        } else {
            false
        }
    });

    let encrypted_pkt_len = aes_cbc_sha256_encrypt(&decrypted_pkt[..(decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH)], &(*esp_hdr), payload).unwrap();

    if match_res{   
        return Ok(Some(v4));
    }
    else{
        return Ok(None);
    }

    Ok(Some(v4))
}

fn main() -> Result<()> {
    let configuration = load_config()?;
    println!("{}", configuration);
    let mut runtime = Runtime::init(&configuration)?;
    runtime.add_pipeline_to_run(install);
    runtime.execute()
}

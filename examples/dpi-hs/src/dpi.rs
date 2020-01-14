extern crate hyperscan;
extern crate aho_corasick;
use netbricks::common::Result;
use netbricks::packets::ip::v4::Ipv4;
use netbricks::packets::{Ethernet, Packet, RawPacket, Tcp};
use std::str;
use std::io::stdout;
use std::io;
use aho_corasick::AhoCorasick;
use std::cell::RefCell;
use std::io::{BufRead, BufReader};
use hyperscan::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use netbricks::utils::HSDPIRULES;

const RULE_NUM: usize = (1 << 30); 

fn parse_file() -> Result<Patterns> {
    let mut rules = vec![];

    for line in HSDPIRULES.iter() {
        rules.push(line);
    }
    if RULE_NUM < rules.len() {
        rules.truncate(RULE_NUM);
    }
    let patterns = rules.iter()
        .filter_map(|line| -> Option<Pattern> {
            let line = line.trim();

            if line.len() > 0 && !line.starts_with('#') {
                if let Ok(pattern) = Pattern::parse(line) {
                    return Some(pattern);
                }
            }
            None
        });

    Ok(patterns.collect())
}

pub struct HSC {
    /// Hyperscan compiled database (block mode)
    pub db_block: BlockDatabase,
    /// Hyperscan temporary scratch space (used in both modes)
    pub scratch: RawScratch,
    // Count of matches found during scanning
    pub match_count: AtomicUsize,
}
impl HSC {
    fn new(db_block: BlockDatabase) -> Result<HSC> {
        let scratch = db_block.alloc().unwrap();
        Ok(HSC {
            db_block: db_block,
            scratch: scratch,
            match_count: AtomicUsize::new(0),
        })
    }

    fn on_match(_: u32, _: u64, _: u64, _: u32, match_count: &AtomicUsize) -> u32 {
        match_count.fetch_add(1, Ordering::Relaxed);
        0
    }

    // Scan each packet (in the ordering given in the PCAP file)
    // through Hyperscan using the block-mode interface.
    fn scan_block(&mut self, payload: &[u8]) {
        if let Err(err) = self.db_block.scan(
            payload,
            0,
            &self.scratch,
            Some(Self::on_match),
            Some(&self.match_count),
        ) {
            println!("ERROR: Unable to scan packet. Exiting. {}", err)
        }
    }
}
/* According to my customized pktgen_zeroloss: */
// set pkt_size: 48 includes the 4B pkt_idx, 2B burst_size, and 2B identifier;
// int pkt_size = 48 + sizeof(struct ether_hdr); // 48 + 14 = 62 bytes
// const PAYLOAD_OFFSET: usize = 62; // payload offset relative to the ethernet header.

thread_local! {
    pub static HYPERSCAN: RefCell<HSC> = {
        // do the actual file reading and string handling
        let patterns = parse_file().unwrap();
        println!("Compiling Hyperscan databases with {} patterns.", patterns.len());
        let db = patterns.build().unwrap();
        RefCell::new(HSC::new(db).unwrap())
    };
}

pub fn dpi(packet: RawPacket) -> Result<Tcp<Ipv4>> {
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let tcp = v4.parse::<Tcp<Ipv4>>()?;
    let payload: &[u8] = tcp.get_payload();

    // println!("{}", payload.len());
    // stdout().flush().unwrap();
    
    // let payload_str = match str::from_utf8(&payload[..]) {
    //     Ok(v) => v,
    //     Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    // };
    // from_utf8_unchecked

    // println!("{}", payload_str);
    // stdout().flush().unwrap();

    // let mut matches = vec![];
    // AC.with(|ac| {
    //     for mat in ac.borrow().find_iter(payload) {
    //         matches.push((mat.pattern(), mat.start(), mat.end()));
    //     }
    // });
    HYPERSCAN.with(|hc| {
        hc.borrow_mut().scan_block(payload)
    });
    
    // println!("{:?}", matches);
    // stdout().flush().unwrap();

    Ok(tcp)
}

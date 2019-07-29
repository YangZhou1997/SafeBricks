use fnv::FnvHasher;
use netbricks::common::Result;
use netbricks::packets::ip::v4::Ipv4;
use netbricks::packets::{Ethernet, Packet, RawPacket};
use std::collections::HashMap;
use std::convert::From;
use std::hash::BuildHasherDefault;
use std::net::Ipv4Addr;
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use netbricks::utils::ipsec::*;
use std::cell::RefCell;

type FnvHash = BuildHasherDefault<FnvHasher>;

pub struct IPLookup {
    tbl24: Vec<u16>,
    tbl_long: Vec<u16>,
    current_tbl_long: usize,
    raw_entries: Vec<HashMap<u32, u16, FnvHash>>,
}

const TBL24_SIZE: usize = ((1 << 24) + 1);
const RAW_SIZE: usize = 33;
const OVERFLOW_MASK: u16 = 0x8000;
pub const GATE_NUM: u16 = 256;

#[derive(Default, Clone)]
struct Empty;
impl Default for IPLookup {
    fn default() -> IPLookup {
        IPLookup {
            tbl24: (0..TBL24_SIZE).map(|_| 0).collect(),
            tbl_long: (0..TBL24_SIZE).map(|_| 0).collect(),
            current_tbl_long: 0,
            raw_entries: (0..RAW_SIZE).map(|_| Default::default()).collect(),
        }
    }
}

impl IPLookup {
    pub fn new() -> IPLookup {
        Default::default()
    }

    pub fn insert_ipv4(&mut self, ip: Ipv4Addr, len: usize, gate: u16) {
        let ip_u32 = u32::from(ip);
        self.insert(ip_u32, len, gate);
    }

    pub fn insert(&mut self, ip: u32, len: usize, gate: u16) {
        self.raw_entries[len].insert(ip, gate);
    }

    pub fn construct_table(&mut self) {
        for i in 0..25 {
            for (k, v) in &self.raw_entries[i] {
                let start = (k >> 8) as usize;
                let end = (start + (1 << (24 - i))) as usize;
                for pfx in start..end {
                    self.tbl24[pfx] = *v;
                }
            }
        }
        for i in 25..RAW_SIZE {
            for (k, v) in &self.raw_entries[i] {
                let addr = *k as usize;
                let t24entry = self.tbl24[addr >> 8];
                if (t24entry & OVERFLOW_MASK) == 0 {
                    // Not overflown and entered yet
                    let ctlb = self.current_tbl_long;
                    let start = ctlb + (addr & 0xff); // Look at last 8 bits (since first 24 are predetermined.
                    let end = start + (1 << (32 - i));
                    for j in ctlb..(ctlb + 256) {
                        if j < start || j >= end {
                            self.tbl_long[j] = t24entry;
                        } else {
                            self.tbl_long[j] = *v;
                        }
                    }
                    self.tbl24[addr >> 8] = ((ctlb >> 8) as u16) | OVERFLOW_MASK;
                    self.current_tbl_long += 256;
                } else {
                    let start = (((t24entry & (!OVERFLOW_MASK)) as usize) << 8) + (addr & 0xff);
                    let end = start + (1 << (32 - i));
                    for j in start..end {
                        self.tbl_long[j] = *v;
                    }
                }
            }
        }
    }

    #[inline]
    pub fn lookup_entry(&self, ip: Ipv4Addr) -> u16 {
        let addr = u32::from(ip) as usize;
        let t24entry = self.tbl24[addr >> 8];
        if (t24entry & OVERFLOW_MASK) > 0 {
            let index = (((t24entry & !OVERFLOW_MASK) as usize) << 8) + (addr & 0xff);
            self.tbl_long[index]
        } else {
            t24entry
        }
    }
}

thread_local! {
    pub static LOOKUP_TABLE: RefCell<IPLookup> = {
        let mut rng = thread_rng();
        let mut lpm_table = IPLookup::new();

        for _ in 1..100 {
            let a: u8 = rng.sample(Uniform::new_inclusive(0, 255));
            let b: u8 = rng.sample(Uniform::new_inclusive(0, 255));
            let c: u8 = rng.sample(Uniform::new_inclusive(0, 255));
            let d: u8 = rng.sample(Uniform::new_inclusive(0, 255));
            let port: u16 = rng.sample(Uniform::new_inclusive(0, GATE_NUM - 1));
            lpm_table.insert_ipv4(Ipv4Addr::new(a, b, c, d), 32, port);
        }

        lpm_table.construct_table();
        RefCell::new(lpm_table)
    };
}

thread_local! {
    pub static COUNT_PORTS: RefCell<Vec<u32>> = {
        let count_ports = (0..GATE_NUM).map(|_| 0).collect();
        RefCell::new(count_ports)
    };
}

pub fn lpm(packet: RawPacket) -> Result<Ipv4> {
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let payload: &mut [u8] = v4.get_payload_mut(); // payload.len()

    let esp_hdr: &mut [u8] = &mut [0u8; 8];
    esp_hdr.copy_from_slice(&payload[0..ESP_HEADER_LENGTH]);

    let decrypted_pkt: &mut [u8] = &mut [0u8; 2000];
    // let decrypted_pkt_len = aes_cbc_sha256_decrypt(payload, decrypted_pkt, false).unwrap();
    let decrypted_pkt_len = aes_gcm128_decrypt_openssl(payload, decrypted_pkt, false).unwrap();
    // let decrypted_pkt_len = aes_gcm128_decrypt_mbedtls(payload, decrypted_pkt, false).unwrap();

    let srcip = get_src_ip(decrypted_pkt);
    let port = LOOKUP_TABLE.with(|lookup_table| {
        lookup_table.borrow().lookup_entry(srcip) as u32
    });
    COUNT_PORTS.with(|count_ports| {
        (*count_ports.borrow_mut())[port as usize] += 1;
    });

    // let encrypted_pkt_len = aes_cbc_sha256_encrypt(&decrypted_pkt[..(decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH)], &(*esp_hdr), payload).unwrap();
    let encrypted_pkt_len = aes_gcm128_encrypt_openssl(&decrypted_pkt[..(decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH)], &(*esp_hdr), payload).unwrap();
    // let encrypted_pkt_len = aes_gcm128_encrypt_mbedtls(&decrypted_pkt[..(decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH)], &(*esp_hdr), payload).unwrap();

    Ok(v4)
}

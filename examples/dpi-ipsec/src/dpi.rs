extern crate aho_corasick;
use netbricks::common::Result;
use netbricks::packets::ip::v4::Ipv4;
use netbricks::packets::{Ethernet, Packet, RawPacket, Tcp};
use netbricks::utils::ipsec::*;
use std::str;
use std::io::stdout;
use std::io::Write;
use aho_corasick::AhoCorasick;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::cell::RefCell;

const RULE_NUM: usize = (1 << 30); 

/* According to my customized pktgen_zeroloss: */
// set pkt_size: 48 includes the 4B pkt_idx, 2B burst_size, and 2B identifier;
// int pkt_size = 48 + sizeof(struct ether_hdr); // 48 + 14 = 62 bytes
// const PAYLOAD_OFFSET: usize = 62; // payload offset relative to the ethernet header.

thread_local! {
    pub static AC: RefCell<AhoCorasick> = {
        let mut rules = vec![];

        let file = File::open("./dpi/wordrules/word.rules").expect("cannot open file");
        let file = BufReader::new(file);
        for line in file.lines().filter_map(|result| result.ok()){
            // println!("{}", line);
            rules.push(line);
            if rules.len() == RULE_NUM {
                break;
            }
        }

        //let patterns = &["This is", "Yang", "abcedf"];
        let patterns = &rules;
        let m = AhoCorasick::new(patterns);
        RefCell::new(m)
    };
}

pub fn dpi(packet: RawPacket) -> Result<Ipv4> {
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let payload: &mut [u8] = v4.get_payload_mut(); // payload.len()

    // let payload_str = match str::from_utf8(&payload[20..]) {
    //     Ok(v) => v,
    //     Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    // };
    // println!("{}", payload_str);
    // stdout().flush().unwrap();

    let esp_hdr: &mut [u8] = &mut [0u8; 8];
    esp_hdr.copy_from_slice(&payload[0..ESP_HEADER_LENGTH]);

    let decrypted_pkt: &mut [u8] = &mut [0u8; 2000];
    let decrypted_pkt_len = aes_cbc_sha256_decrypt(payload, decrypted_pkt, false).unwrap();
    // println!("decrypted_pkt_len: {}", decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH);
    // stdout().flush().unwrap();

    let mut matches = vec![];
    AC.with(|ac| {
        for mat in ac.borrow().find_iter(&decrypted_pkt[40..(decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH)]) {
            matches.push((mat.pattern(), mat.start(), mat.end()));
        }
    });
    // println!("{:?}", matches);
    // stdout().flush().unwrap();

    let encrypted_pkt_len = aes_cbc_sha256_encrypt(&decrypted_pkt[..(decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH)], &(*esp_hdr), payload).unwrap();
    // println!("encrypted_pkt_len: {}", encrypted_pkt_len);
    // stdout().flush().unwrap();

    Ok(v4)
}

use openssl::symm::*;
use openssl::hash::MessageDigest;
use openssl::memcmp;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use openssl::error::ErrorStack;

use std::io::stdout;
use std::io::Write;

use packets::ip::Flow;
use packets::buffer;
use packets::TcpHeader;
use packets::ip::ProtocolNumbers;
use packets::ip::v4::Ipv4Header;
use std::net::{IpAddr, Ipv4Addr};


#[derive(Debug)]
pub enum CryptoError {
    HmacMismatch,
    PktlenError,
    AESEncryptError,
    AESDecryptError,
}

const AES_KEY: &[u8] = b"\x92\x65\x49\x29\x1f\x40\x1a\xcc\x98\x00\x77\x69\x13\xfd\xc0\x11";
const AES_IV: &[u8] = b"\x31\xa5\xcf\xe1\x05\x30\xb0\x2e\x9c\x5e\xeb\x31\x6f\x4e\x05\x01";
const SHA_KEY: &[u8] = b"\x8a\xcf\xe8\x19\x14\x87\x40\x59\x9d\xd0\xb1\xb1\x20\x1a\xf5\x15\
                  \x53\x1b\x0f\xbc\xf1\x38\xc1\x25\x4c\xf8\xc8\xae\x33\x6d\xc4\xbd";

pub const MAX_PKT_SIZE: usize = 65535;
pub const ESP_HEADER_LENGTH: usize = 8;
pub const AES_CBC_IV_LENGTH: usize = 16;
pub const ESP_HMAC_LEN: usize = 12;
pub const IP_HEADER_LENGTH: usize = 20;
pub const ICV_LEN_SHA256: usize = 16;

// pktptr points to the start of the cleartext ip header.
// after output, output points to the start of the ESP header
// This function will return outlen: u16
pub fn aes_cbc_sha256_encrypt(pktptr: &[u8], esphdr: &[u8], output: &mut [u8]) -> Result<usize, CryptoError>
{
    let pktlen = pktptr.len();
    if (pktlen < 16) || (pktlen%16 != 0) {
        println!("Encrypt: packetlen is not proper");
        stdout().flush().unwrap();
        return Err(CryptoError::PktlenError);
    }
    if pktlen >(MAX_PKT_SIZE - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH - ICV_LEN_SHA256) as usize
    {
        println!("Packet is too big to handle");
        stdout().flush().unwrap();
        return Err(CryptoError::PktlenError);
    }
    output[..ESP_HEADER_LENGTH].copy_from_slice(esphdr);
    output[ESP_HEADER_LENGTH..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)].copy_from_slice(AES_IV);

    let cipher = Cipher::aes_128_cbc();
    let ciphertext = my_encrypt(cipher, AES_KEY, Some(AES_IV), pktptr).unwrap();
    let ciphertext_len = ciphertext.len();
    output[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len)].copy_from_slice(&ciphertext[..]);
    if ciphertext_len != pktlen
    {
        println!("cleartext pktlen: {} vs. ciphertext pktlen: {}", pktptr.len(), ciphertext_len);
        println!("AES encryption errors");
        stdout().flush().unwrap();
        return Err(CryptoError::AESEncryptError);
    }

    let key = PKey::hmac(SHA_KEY).unwrap();
    let mut signer = Signer::new(MessageDigest::sha256(), &key).unwrap();
    signer.update(&output[..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len)]).unwrap();
    let hmac = signer.sign_to_vec().unwrap();
    output[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len)..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len + ICV_LEN_SHA256)].copy_from_slice(&hmac[..ICV_LEN_SHA256]);
    
    Ok(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len + ICV_LEN_SHA256)
}


// pktptr points to the start of the cleartext ip header.
// after output, output points to the start of the ESP header
// This function will return outlen: u16
pub fn aes_cbc_sha256_encrypt_opt(pktptr: &mut [u8], esphdr: &[u8]) -> Result<usize, CryptoError>
{
     // decrypted_pkt_len + ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ICV_LEN_SHA256
    let pktlen = pktptr.len() - ICV_LEN_SHA256 - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH;
    if (pktlen < 16) || (pktlen%16 != 0) {
        println!("Encrypt: packetlen is not proper");
        stdout().flush().unwrap();
        return Err(CryptoError::PktlenError);
    }
    if pktlen >(MAX_PKT_SIZE - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH - ICV_LEN_SHA256) as usize
    {
        println!("Packet is too big to handle");
        stdout().flush().unwrap();
        return Err(CryptoError::PktlenError);
    }

    let cipher = Cipher::aes_128_cbc();
    let ciphertext = my_encrypt(cipher, AES_KEY, Some(AES_IV), pktptr).unwrap();
    let ciphertext_len = ciphertext.len();

    pktptr[..ESP_HEADER_LENGTH].copy_from_slice(esphdr);
    pktptr[ESP_HEADER_LENGTH..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)].copy_from_slice(AES_IV);
    pktptr[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len)].copy_from_slice(&ciphertext[..]);
    
    if ciphertext_len != pktlen
    {
        println!("cleartext pktlen: {} vs. ciphertext pktlen: {}", pktlen, ciphertext_len);
        println!("AES encryption errors");
        stdout().flush().unwrap();
        return Err(CryptoError::AESEncryptError);
    }

    let key = PKey::hmac(SHA_KEY).unwrap();
    let mut signer = Signer::new(MessageDigest::sha256(), &key).unwrap();
    signer.update(&pktptr[..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len)]).unwrap();
    let hmac = signer.sign_to_vec().unwrap();
    pktptr[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len)..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len + ICV_LEN_SHA256)].copy_from_slice(&hmac[..ICV_LEN_SHA256]);
    
    Ok(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len + ICV_LEN_SHA256)
}


fn my_cipher(
    t: Cipher,
    mode: Mode,
    key: &[u8],
    iv: Option<&[u8]>,
    data: &[u8],
) -> Result<Vec<u8>, ErrorStack> {
    let mut c = Crypter::new(t, mode, key, iv)?;
    c.pad(false);
    let mut out = vec![0; data.len() + t.block_size()];
    let count = c.update(data, &mut out)?;
    let rest = c.finalize(&mut out[count..])?;
    out.truncate(count + rest);
    Ok(out)
}


pub fn my_decrypt(
    t: Cipher,
    key: &[u8],
    iv: Option<&[u8]>,
    data: &[u8],
) -> Result<Vec<u8>, ErrorStack> {
    my_cipher(t, Mode::Decrypt, key, iv, data)
}

pub fn my_encrypt(
    t: Cipher,
    key: &[u8],
    iv: Option<&[u8]>,
    data: &[u8],
) -> Result<Vec<u8>, ErrorStack> {
    my_cipher(t, Mode::Encrypt, key, iv, data)
}

// pktptr points to the start of the ESP header
// after calling, output points to the start of the decrypted ip header.
// This function will return outlen: u16
pub fn aes_cbc_sha256_decrypt(pktptr: &[u8], output: &mut [u8], compdigest: bool) -> Result<usize, CryptoError> 
{
    let pktlen = pktptr.len();    
    if pktlen < (ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ICV_LEN_SHA256) {
        println!("Decrypt: Packet length is not proper");
        stdout().flush().unwrap();
        return Err(CryptoError::PktlenError);
    }
    let key = PKey::hmac(SHA_KEY).unwrap();
    let mut signer = Signer::new(MessageDigest::sha256(), &key).unwrap();
    signer.update(&pktptr[..(pktlen - ICV_LEN_SHA256)]).unwrap();
    let hmac = signer.sign_to_vec().unwrap();
    if compdigest
    {
        if !memcmp::eq(&hmac[..ICV_LEN_SHA256], &pktptr[(pktlen - ICV_LEN_SHA256)..])
        {
            println!("INBOUND Mac Mismatch");
            stdout().flush().unwrap();
            return Err(CryptoError::HmacMismatch);
        }
    }

    let cipher = Cipher::aes_128_cbc();
    // let cleartext = decrypt(cipher, AES_KEY, Some(AES_IV), &pktptr[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)..(pktlen - ICV_LEN_SHA256)]).unwrap();
    let cleartext = my_decrypt(cipher, AES_KEY, Some(AES_IV), &pktptr[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)..(pktlen - ICV_LEN_SHA256)]).unwrap();
    
    let cleartext_len = cleartext.len();
    if cleartext_len != pktlen - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH - ICV_LEN_SHA256
    {
        println!("ciphertext pktlen: {} vs. cleartext pktlen: {}", pktlen - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH - ICV_LEN_SHA256, cleartext_len);
        println!("AES decryption errors");
        stdout().flush().unwrap();
        return Err(CryptoError::AESDecryptError);
    }
    output[..(cleartext_len)].copy_from_slice(&cleartext[..]);

    Ok(cleartext_len + ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)
}



// pktptr points to the start of the ESP header
// after calling, output points to the start of the decrypted ip header.
// This function will return outlen: u16
pub fn aes_cbc_sha256_decrypt_opt(pktptr: &mut [u8], compdigest: bool) -> Result<usize, CryptoError> 
{
    let pktlen = pktptr.len();    
    if pktlen < (ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ICV_LEN_SHA256) {
        println!("Decrypt: Packet length is not proper");
        stdout().flush().unwrap();
        return Err(CryptoError::PktlenError);
    }
    let key = PKey::hmac(SHA_KEY).unwrap();
    let mut signer = Signer::new(MessageDigest::sha256(), &key).unwrap();
    signer.update(&pktptr[..(pktlen - ICV_LEN_SHA256)]).unwrap();
    let hmac = signer.sign_to_vec().unwrap();
    if compdigest
    {
        if !memcmp::eq(&hmac[..ICV_LEN_SHA256], &pktptr[(pktlen - ICV_LEN_SHA256)..])
        {
            println!("INBOUND Mac Mismatch");
            stdout().flush().unwrap();
            return Err(CryptoError::HmacMismatch);
        }
    }

    let cipher = Cipher::aes_128_cbc();
    // let cleartext = decrypt(cipher, AES_KEY, Some(AES_IV), &pktptr[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)..(pktlen - ICV_LEN_SHA256)]).unwrap();
    let cleartext = my_decrypt(cipher, AES_KEY, Some(AES_IV), &pktptr[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)..(pktlen - ICV_LEN_SHA256)]).unwrap();
    
    let cleartext_len = cleartext.len();
    if cleartext_len != pktlen - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH - ICV_LEN_SHA256
    {
        println!("ciphertext pktlen: {} vs. cleartext pktlen: {}", pktlen - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH - ICV_LEN_SHA256, cleartext_len);
        println!("AES decryption errors");
        stdout().flush().unwrap();
        return Err(CryptoError::AESDecryptError);
    }
    pktptr[..(cleartext_len)].copy_from_slice(&cleartext[..]);

    Ok(cleartext_len + ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)
}


pub fn get_flow(pkt: &[u8]) -> Flow{
    unsafe {
        let ip_hdr: *const Ipv4Header = (&pkt[0] as *const u8) as *const Ipv4Header;
        let tcp_hdr: *const TcpHeader = (&pkt[0] as *const u8).offset(20) as *const TcpHeader;
        Flow::new(
            IpAddr::V4((*ip_hdr).src()),
            IpAddr::V4((*ip_hdr).dst()),
            (*tcp_hdr).src_port(),
            (*tcp_hdr).dst_port(),
            ProtocolNumbers::Tcp,
        )
    }
}


pub fn get_src_ip(pkt: &[u8]) -> Ipv4Addr{
    unsafe {
        let ip_hdr: *const Ipv4Header = (&pkt[0] as *const u8) as *const Ipv4Header;
        (*ip_hdr).src()
    }
}


// for 
pub fn set_dst_ip(pkt: &mut [u8], dst_ip: u32){
    unsafe {
        let ip_hdr: *mut Ipv4Header = (&mut pkt[0] as *mut u8) as *mut Ipv4Header;
        (*ip_hdr).set_dst(Ipv4Addr::new(((dst_ip >> 24) & 0xFF) as u8,
             ((dst_ip >> 16) & 0xFF) as u8, ((dst_ip >> 8) & 0xFF) as u8, (dst_ip & 0xFF) as u8));
    }
}


pub fn set_flow(pkt: &mut [u8], flow: Flow){
    unsafe {
        let ip_hdr: *mut Ipv4Header = (&mut pkt[0] as *mut u8) as *mut Ipv4Header;
        let tcp_hdr: *mut TcpHeader = (&mut pkt[0] as *mut u8).offset(20) as *mut TcpHeader;
        
        if let IpAddr::V4(ipv4) = flow.src_ip() {
            (*ip_hdr).set_src(ipv4);
        }
        if let IpAddr::V4(ipv4) = flow.dst_ip() {
            (*ip_hdr).set_dst(ipv4);
        }
        (*tcp_hdr).set_src_port(flow.src_port());
        (*tcp_hdr).set_dst_port(flow.dst_port());
        (*ip_hdr).set_protocol(ProtocolNumbers::Tcp);
    }
}
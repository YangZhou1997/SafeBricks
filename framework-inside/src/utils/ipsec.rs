use mbedtls::cipher::raw::Cipher as CipherMbed;
use mbedtls::cipher::raw;
use mbedtls::cipher::raw::Operation;
use mbedtls::hash::Md;
use mbedtls::hash::Type;

use std::io::stdout;
use std::io::Write;
use std::cmp::Eq;

use packets::ip::Flow;
use packets::buffer;
use packets::TcpHeader;
use packets::ip::ProtocolNumbers;
use packets::ip::v4::Ipv4Header;
use std::net::{IpAddr, Ipv4Addr};
use std::cell::RefCell;


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

pub const AES_GCM_IV_LENGTH: usize = 16;
pub const ICV_LEN_GCM128: usize = 16;

thread_local! {
    pub static CIPHER_ENCRY_CBC_SHA: RefCell<CipherMbed> = {
        let mut cipher = CipherMbed::setup(
            raw::CipherId::Aes,
            raw::CipherMode::CBC,
            (AES_KEY.len() * 8) as u32,
        ).unwrap();
        cipher.set_key(Operation::Encrypt, AES_KEY).unwrap();
        cipher.set_iv(AES_IV).unwrap();
        RefCell::new(cipher)
    };
}

thread_local! {
    pub static CIPHER_DECRY_CBC_SHA: RefCell<CipherMbed> = {
        let mut cipher = CipherMbed::setup(
            raw::CipherId::Aes,
            raw::CipherMode::GCM,
            (AES_KEY.len() * 8) as u32,
        ).unwrap();
        cipher.set_key(Operation::Decrypt, AES_KEY).unwrap();
        cipher.set_iv(AES_IV).unwrap();
        RefCell::new(cipher)
    };
}

// pktptr points to the start of the cleartext ip header.
// after output, output points to the start of the ESP header
// This function will return outlen: u16
pub fn aes_cbc_sha256_encrypt_mbedtls(pktptr: &[u8], esphdr: &[u8], output: &mut [u8]) -> Result<usize, CryptoError>
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

    CIPHER_ENCRY_CBC_SHA.with(|cipher| {
        let mut cipher_lived = cipher.borrow_mut();
        let ciphertext_len = cipher_lived.encrypt(pktptr, 
            &mut output[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + pktlen)]).unwrap();
        if ciphertext_len != pktlen
        {
            println!("cleartext pktlen: {} vs. ciphertext pktlen: {}", pktptr.len(), ciphertext_len);
            println!("AES encryption errors");
            stdout().flush().unwrap();
            return Err(CryptoError::AESEncryptError);
        }
        let hmac: &mut [u8] = &mut [0u8; 16];
        Md::hmac(Type::Sha256, SHA_KEY, &output[..(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len)], hmac).unwrap();
        output[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len)..].copy_from_slice(hmac);
        Ok(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ciphertext_len + ICV_LEN_SHA256)
    })
}

// pktptr points to the start of the ESP header
// after calling, output points to the start of the decrypted ip header.
// This function will return outlen: u16
pub fn aes_cbc_sha256_decrypt_mbedtls(pktptr: &[u8], output: &mut [u8], compdigest: bool) -> Result<usize, CryptoError> 
{
    let pktlen = pktptr.len();    
    if pktlen < (ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ICV_LEN_SHA256) {
        println!("Decrypt: Packet length is not proper");
        stdout().flush().unwrap();
        return Err(CryptoError::PktlenError);
    }
    let hmac: &mut [u8] = &mut [0u8; 16];
    Md::hmac(Type::Sha256, SHA_KEY, &pktptr[..(pktlen - ICV_LEN_SHA256)], hmac).unwrap();
    
    if compdigest
    {
        if !(&hmac[..ICV_LEN_SHA256] == &pktptr[(pktlen - ICV_LEN_SHA256)..])
        {
            println!("INBOUND Mac Mismatch");
            stdout().flush().unwrap();
            return Err(CryptoError::HmacMismatch);
        }
    }

    CIPHER_DECRY_CBC_SHA.with(|cipher| {
        let mut cipher = cipher.borrow_mut();
        if let Ok(cleartext_len) = cipher.decrypt(&pktptr[(ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH)..(pktlen - ICV_LEN_SHA256)],
            &mut output[..(pktlen - (ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH + ICV_LEN_SHA256))])
        {
            if cleartext_len != pktlen - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH - ICV_LEN_SHA256
            {
                println!("ciphertext pktlen: {} vs. cleartext pktlen: {}", pktlen - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH - ICV_LEN_SHA256, cleartext_len);
                println!("AES decryption errors");
                stdout().flush().unwrap();
                return Err(CryptoError::AESDecryptError);
            }
            return Ok(cleartext_len + ESP_HEADER_LENGTH + AES_CBC_IV_LENGTH);
        }
        return Ok(pktlen - ICV_LEN_SHA256);  
    })
}


thread_local! {
    pub static CIPHER_ENCRY_GCM: RefCell<CipherMbed> = {
        let mut cipher = CipherMbed::setup(
            raw::CipherId::Aes,
            raw::CipherMode::GCM,
            (AES_KEY.len() * 8) as u32,
        ).unwrap();
        cipher.set_key(Operation::Encrypt, AES_KEY).unwrap();
        cipher.set_iv(AES_IV).unwrap();
        RefCell::new(cipher)
    };
}

thread_local! {
    pub static CIPHER_DECRY_GCM: RefCell<CipherMbed> = {
        let mut cipher = CipherMbed::setup(
            raw::CipherId::Aes,
            raw::CipherMode::GCM,
            (AES_KEY.len() * 8) as u32,
        ).unwrap();
        cipher.set_key(Operation::Decrypt, AES_KEY).unwrap();
        cipher.set_iv(AES_IV).unwrap();
        RefCell::new(cipher)
    };
}

pub fn aes_gcm128_encrypt_mbedtls(pktptr: &[u8], esphdr: &[u8], output: &mut [u8]) -> Result<usize, CryptoError>
{
    let pktlen = pktptr.len();
    // if pktlen >(MAX_PKT_SIZE - ESP_HEADER_LENGTH - AES_GCM_IV_LENGTH - ICV_LEN_GCM128) as usize
    // {
    //     println!("Packet is too big to handle");
    //     stdout().flush().unwrap();
    //     return Err(CryptoError::PktlenError);
    // }
    let hmac: &mut [u8] = &mut [0u8; 16];
    let aad: &mut [u8] = &mut [0u8; (ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH)];
    aad[..ESP_HEADER_LENGTH].copy_from_slice(esphdr);
    aad[ESP_HEADER_LENGTH..(ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH)].copy_from_slice(AES_IV);
    
    CIPHER_ENCRY_GCM.with(|cipher| {
        let mut cipher_lived = cipher.borrow_mut();
        cipher_lived.encrypt_auth(aad, pktptr, 
            &mut output[(ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH)..(ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH + pktlen)], hmac).unwrap();
    });
    
    output[..(ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH)].copy_from_slice(aad);
    output[(ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH + pktlen)..].copy_from_slice(hmac);
    
    Ok(ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH + pktlen + ICV_LEN_GCM128)
}

pub fn aes_gcm128_decrypt_mbedtls(pktptr: &[u8], output: &mut [u8], compdigest: bool) -> Result<usize, CryptoError>
{
    let pktlen = pktptr.len();    
    // if pktlen < (ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH + ICV_LEN_GCM128) {
    //     println!("Decrypt: Packet length is not proper");
    //     stdout().flush().unwrap();
    //     return Err(CryptoError::PktlenError);
    // }
    CIPHER_DECRY_GCM.with(|cipher| {
        let mut cipher = cipher.borrow_mut();
        if let Ok(_plain_text) = cipher.decrypt_auth(&pktptr[0..(ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH)], &pktptr[(ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH)..(pktlen - ICV_LEN_GCM128)],
            &mut output[..(pktlen - (ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH + ICV_LEN_GCM128))], &pktptr[(pktlen - ICV_LEN_GCM128)..])
        {
            let cleartext_len = pktlen - ESP_HEADER_LENGTH - AES_GCM_IV_LENGTH - ICV_LEN_GCM128;
            return Ok(cleartext_len + ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH);
        }
        return Ok(pktlen - ICV_LEN_GCM128);  
    })
}




#[inline]
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


#[inline]
pub fn get_src_ip(pkt: &[u8]) -> Ipv4Addr{
    unsafe {
        let ip_hdr: *const Ipv4Header = (&pkt[0] as *const u8) as *const Ipv4Header;
        (*ip_hdr).src()
    }
}


#[inline]
pub fn set_dst_ip(pkt: &mut [u8], dst_ip: u32){
    unsafe {
        let ip_hdr: *mut Ipv4Header = (&mut pkt[0] as *mut u8) as *mut Ipv4Header;
        (*ip_hdr).set_dst(Ipv4Addr::new(((dst_ip >> 24) & 0xFF) as u8,
             ((dst_ip >> 16) & 0xFF) as u8, ((dst_ip >> 8) & 0xFF) as u8, (dst_ip & 0xFF) as u8));
    }
}


#[inline]
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
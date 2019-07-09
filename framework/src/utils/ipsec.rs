use openssl::symm::{encrypt, Cipher};

use openssl::hash::MessageDigest;
use openssl::memcmp;
use openssl::pkey::PKey;
use openssl::sign::Signer;

// Mathematical "errors" we want to catch
#[derive(Debug)]
pub enum CryptoError {
    HmacMismatch,
    PktlenError,
}

const AES_KEY: &[u8] = b"\x92\x65\x49\x29\x1f\x40\x1a\xcc\x98\x00\x77\x69\x13\xfd\xc0\x11";
const AES_IV: &[u8] = b"\x31\xa5\xcf\xe1\x05\x30\xb0\x2e\x9c\x5e\xeb\x31\x6f\x4e\x05\x01";
const SHA_KEY: &[u8] = b"\x8a\xcf\xe8\x19\x14\x87\x40\x59\x9d\xd0\xb1\xb1\x20\x1a\xf5\x15\
                  \x53\x1b\x0f\xbc\xf1\x38\xc1\x25\x4c\xf8\xc8\xae\x33\x6d\xc4\xbd";

const MAX_PKT_SIZE: u16 = 65535;
const ESP_HEADER_LENGTH: u16 = 8;
const AES_CBC_IV_LENGTH: u16 = 16;
const ESP_HMAC_LEN: u16 = 12;
const IP_HEADER_LENGTH: u16 = 20;
const ICV_LEN_SHA256: u16 = 16;


// int AES_cbc_sha256_encrypt(
//     uint16_t aes_key_len, uint8_t * aes_key,
//     uint16_t sha1_keylen, uint8_t * sha1_key,
//     uint8_t * espheader, uint8_t * aes_iv,
//     uint8_t * pktptr, uint16_t pktlen,
//     uint8_t * outptr, uint16_t * outlen
//     )

// pktptr points to the start of the cleartext ip header.
// after output, pktptr points to the start of the ESP header
// This function will return outlen: u16
pub fn AES_cbc_sha256_encrypt(pktptr: &[u8], output: &mut [u8]) -> Result<u16, CryptoError>
{
    let pktlen = pktptr.len();
    if((pktlen < 16) || (pktlen%16 != 0)) {
        println!("packetlen is not proper");
        return Err(CryptoError::PktlenError);
    }
    if(pktlen >(MAX_PKT_SIZE - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH - ICV_LEN_SHA256) as usize)
    {
        println!("Packet is too big to handle");
        return Err(CryptoError::PktlenError);
    }

    let data = b"hello, world!";
    let data2 = b"hola, mundo!";

    let key = PKey::hmac(SHA_KEY).unwrap();
    let mut signer = Signer::new(MessageDigest::sha256(), &key).unwrap();
    signer.update(data).unwrap();
    signer.update(data2).unwrap();
    let hmac = signer.sign_to_vec().unwrap();


    let cipher = Cipher::aes_128_cbc();
    // let data = b"Some Crypto Text";
    let ciphertext = encrypt(cipher, AES_KEY, Some(AES_IV), pktptr).unwrap();

    // assert_eq!(b"\xB4\xB9\xE7\x30\xD6\xD6\xF7\xDE\x77\x3F\x1C\xFF\xB3\x3E\x44\x5A\x91\xD7\x27\x62\x87\x4D
                //  \xFB\x3C\x5E\xC4\x59\x72\x4A\xF4\x7C\xA1", &ciphertext[..]);

    // Create a PKey

    // Compute the HMAC
    let mut signer = Signer::new(MessageDigest::sha256(), &key).unwrap();
    signer.update(data).unwrap();
    signer.update(data2).unwrap();
    let hmac = signer.sign_to_vec().unwrap();

    // `Verifier` cannot be used with HMACs; use the `memcmp::eq` function instead
    //
    // Do not simply check for equality with `==`!
    // assert!(memcmp::eq(&hmac, &target));

    Ok(1)
}

// int AES_cbc_sha256_decrypt(
//     uint16_t aes_key_len, uint8_t * aes_key, 
//     uint16_t sha1_keylen, uint8_t * sha1_key, 
//     uint8_t * aes_iv,
//     uint8_t * pktptr, uint16_t pktlen,
//     uint8_t * outptr, uint16_t * outlen,
//     uint8_t compdigest
//     )

// pktptr points to the start of the ESP header
// after calling, output points to the start of the decrypted ip header.
// This function will return outlen: u16
pub fn AES_cbc_sha256_decrypt(pktptr: &[u8], output: &mut [u8], compdigest: u8) -> Result<u16, CryptoError> {
    Ok(1)
}

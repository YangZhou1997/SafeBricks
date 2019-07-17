use common::Result;
use failure::Fail;
use hex;
use native::mbuf::MBuf;
use packets::{buffer, Fixed, Header, Packet, RawPacket};
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;

/* Ethernet Type II Frame

   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
   |  Dst MAC  |  Src MAC  |Typ|             Payload               |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+                                   +
   |                                                               |
   |                                                               |
   |                                                               |
   +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

   Destination MAC      48-bit MAC address of the originator of the
                        packet.

   Source MAC           48-bit MAC address of the intended recipient of
                        the packet.

   Ether Type           16-bit indicator. Identifies which protocol is
                        encapsulated in the payload of the frame.
*/

/// MAC address
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C, packed)]
pub struct MacAddr([u8; 6]);

impl MacAddr {
    pub const UNSPECIFIED: Self = MacAddr([0, 0, 0, 0, 0, 0]);

    #[allow(clippy::many_single_char_names)]
    pub fn new(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) -> Self {
        MacAddr([a, b, c, d, e, f])
    }

    pub fn new_from_slice(slice: &[u8]) -> Self {
        MacAddr([slice[0], slice[1], slice[2], slice[3], slice[4], slice[5]])
    }

    /// Returns the six bytes the MAC address consists of
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn octets(&self) -> [u8; 6] {
        self.0
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

#[derive(Debug, Fail)]
#[fail(display = "Failed to parse '{}' as MAC address.", _0)]
pub struct MacParseError(String);

impl FromStr for MacAddr {
    type Err = MacParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match hex::decode(s.replace(":", "").replace("-", "")) {
            Ok(ref octets) if octets.len() == 6 => Ok(MacAddr::new_from_slice(octets.as_slice())),
            _ => Err(MacParseError(s.to_owned())),
        }
    }
}

impl<'de> Deserialize<'de> for MacAddr {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        MacAddr::from_str(&s).map_err(de::Error::custom)
    }
}

/// The protocol type in the ethernet packet payload
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(C, packed)]
pub struct EtherType(pub u16);

impl EtherType {
    pub fn new(value: u16) -> Self {
        EtherType(value)
    }
}

/// Supported ethernet payload protocol types
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod EtherTypes {
    use super::EtherType;

    // Internet Protocol version 4
    pub const Ipv4: EtherType = EtherType(0x0800);
    // Internet Protocol version 6
    pub const Ipv6: EtherType = EtherType(0x86DD);
}

impl fmt::Display for EtherType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                EtherTypes::Ipv4 => "IPv4".to_string(),
                EtherTypes::Ipv6 => "IPv6".to_string(),
                _ => format!("0x{:04x}", self.0),
            }
        )
    }
}

/// Ethernet header
#[derive(Default, Debug)]
#[repr(C, packed)]
pub struct EthernetHeader {
    dst: MacAddr,
    src: MacAddr,
    ether_type: u16,
}

impl Header for EthernetHeader {}

/// Ethernet packet
#[derive(Debug)]
pub struct Ethernet {
    envelope: RawPacket,
    mbuf: *mut MBuf,
    offset: usize,
    header: *mut EthernetHeader,
}

impl Ethernet {
    #[inline]
    pub fn src(&self) -> MacAddr {
        self.header().src
    }

    #[inline]
    pub fn set_src(&mut self, src: MacAddr) {
        self.header_mut().src = src
    }

    #[inline]
    pub fn dst(&self) -> MacAddr {
        self.header().dst
    }

    #[inline]
    pub fn set_dst(&mut self, dst: MacAddr) {
        self.header_mut().dst = dst
    }

    #[inline]
    pub fn ether_type(&self) -> EtherType {
        EtherType::new(u16::from_be(self.header().ether_type))
    }

    #[inline]
    pub fn set_ether_type(&mut self, ether_type: EtherType) {
        self.header_mut().ether_type = u16::to_be(ether_type.0)
    }

    #[inline]
    pub fn swap_addresses(&mut self) {
        let src = self.src();
        let dst = self.dst();
        self.set_src(dst);
        self.set_dst(src);
    }
}

impl fmt::Display for Ethernet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} > {}, ether_type: {}",
            self.src(),
            self.dst(),
            self.ether_type()
        )
    }
}

impl Packet for Ethernet {
    type Header = EthernetHeader;
    type Envelope = RawPacket;

    #[inline]
    fn envelope(&self) -> &Self::Envelope {
        &self.envelope
    }

    #[inline]
    fn envelope_mut(&mut self) -> &mut Self::Envelope {
        &mut self.envelope
    }

    #[doc(hidden)]
    #[inline]
    fn mbuf(&self) -> *mut MBuf {
        self.mbuf
    }

    #[inline]
    fn offset(&self) -> usize {
        self.offset
    }

    #[doc(hidden)]
    #[inline]
    fn header(&self) -> &Self::Header {
        unsafe { &(*self.header) }
    }

    #[doc(hidden)]
    #[inline]
    fn header_mut(&mut self) -> &mut Self::Header {
        unsafe { &mut (*self.header) }
    }

    #[inline]
    fn header_len(&self) -> usize {
        Self::Header::size()
    }

    #[doc(hidden)]
    #[inline]
    fn do_parse(envelope: Self::Envelope) -> Result<Self> {
        let mbuf = envelope.mbuf();
        let offset = envelope.payload_offset();
        let header = buffer::read_item::<Self::Header>(mbuf, offset)?;

        Ok(Ethernet {
            envelope,
            mbuf,
            offset,
            header,
        })
    }

    #[doc(hidden)]
    #[inline]
    fn do_push(envelope: Self::Envelope) -> Result<Self> {
        let mbuf = envelope.mbuf();
        let offset = envelope.payload_offset();

        buffer::alloc(mbuf, offset, Self::Header::size())?;
        let header = buffer::write_item::<Self::Header>(mbuf, offset, &Default::default())?;

        Ok(Ethernet {
            envelope,
            mbuf,
            offset,
            header,
        })
    }

    #[inline]
    fn remove(self) -> Result<Self::Envelope> {
        buffer::dealloc(self.mbuf, self.offset, self.header_len())?;
        Ok(self.envelope)
    }

    #[inline]
    fn cascade(&mut self) {
        self.envelope_mut().cascade();
    }

    #[inline]
    fn deparse(self) -> Self::Envelope {
        self.envelope
    }
}

use common::Result;
use native::mbuf::MBuf;
use packets::checksum::PseudoHeader;
use packets::ip::{IpAddrMismatchError, IpPacket, ProtocolNumber};
use packets::{buffer, Ethernet, Fixed, Header, Packet};
use std::fmt;
use std::net::{IpAddr, Ipv6Addr};

pub use self::srh::*;
pub mod srh;

/// Common behaviors shared by IPv6 and extension packets
pub trait Ipv6Packet: IpPacket {}

/// The minimum IPv6 MTU
///
/// https://tools.ietf.org/html/rfc2460#section-5
pub const IPV6_MIN_MTU: usize = 1280;

/*  From https://tools.ietf.org/html/rfc8200#section-3
    and https://tools.ietf.org/html/rfc3168 (succeeding traffic class)
    IPv6 Header Format

    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |Version|    DSCP_ECN   |           Flow Label                  |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |         Payload Length        |  Next Header  |   Hop Limit   |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                                                               |
    +                                                               +
    |                                                               |
    +                         Source Address                        +
    |                                                               |
    +                                                               +
    |                                                               |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                                                               |
    +                                                               +
    |                                                               |
    +                      Destination Address                      +
    |                                                               |
    +                                                               +
    |                                                               |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    Version             4-bit Internet Protocol version number = 6.

    DSCP_ECN:           8-bit Differentiated services (via RFC 2474 ~
                        https://tools.ietf.org/html/rfc2474) enhancements to the
                        Internet protocol are intended to enable scalable
                        service discrimination in the Internet without the need
                        for per-flow state and signaling at every hop.  A
                        variety of services may be built from a small,
                        well-defined set of building blocks which are deployed
                        in network nodes. The services may be either end-to-end
                        or intra-domain; they include both those that can
                        satisfy quantitative performance requirements (e.g.,
                        peak bandwidth) and those based on relative performance
                        (e.g., "class" differentiation).

                        Taking the last two bits, is ECN, the addition of
                        Explicit Congestion Notification to IP; RFC-3168
                        (https://tools.ietf.org/html/rfc3168) covers this in
                        detail. This uses an ECN field in the IP header with two
                        bits, making four ECN codepoints, '00' to '11'.  The
                        ECN-Capable Transport (ECT) codepoints '10' and '01' are
                        set by the data sender to indicate that the end-points
                        of the transport protocol are ECN-capable; we call them
                        ECT(0) and ECT(1) respectively.  The phrase "the ECT
                        codepoint" in this documents refers to either of the two
                        ECT codepoints.  Routers treat the ECT(0) and ECT(1)
                        codepoints as equivalent.  Senders are free to use
                        either the ECT(0) or the ECT(1) codepoint to indicate
                        ECT, on a packet-by-packet basis.

    Flow Label          20-bit flow label.

    Payload Length      16-bit unsigned integer.  Length of the IPv6
                        payload, i.e., the rest of the packet following
                        this IPv6 header, in octets.  (Note that any
                        extension headers present are considered part of
                        the payload, i.e., included in the length count.)

    Next Header         8-bit selector.  Identifies the type of header
                        immediately following the IPv6 header.  Uses the
                        same values as the IPv4 Protocol field [RFC-1700
                        et seq.].

    Hop Limit           8-bit unsigned integer.  Decremented by 1 by
                        each node that forwards the packet. The packet
                        is discarded if Hop Limit is decremented to
                        zero.

    Source Address      128-bit address of the originator of the packet.

    Destination Address 128-bit address of the intended recipient of the
                        packet (possibly not the ultimate recipient, if
                        a Routing header is present).
*/

// Masks
const DSCP: u32 = 0x0fc0_0000;
const ECN: u32 = 0x0030_0000;
const FLOW: u32 = 0xfffff;

/// IPv6 header
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Ipv6Header {
    version_to_flow_label: u32,
    payload_length: u16,
    next_header: u8,
    hop_limit: u8,
    src: Ipv6Addr,
    dst: Ipv6Addr,
}

impl Default for Ipv6Header {
    fn default() -> Ipv6Header {
        Ipv6Header {
            version_to_flow_label: u32::to_be(6 << 28),
            payload_length: 0,
            next_header: 0,
            hop_limit: 0,
            src: Ipv6Addr::UNSPECIFIED,
            dst: Ipv6Addr::UNSPECIFIED,
        }
    }
}

impl Header for Ipv6Header {}

/// IPv6 packet
#[derive(Debug)]
pub struct Ipv6 {
    envelope: Ethernet,
    mbuf: *mut MBuf,
    offset: usize,
    header: *mut Ipv6Header,
}

impl Ipv6 {
    #[inline]
    pub fn version(&self) -> u8 {
        // Protocol Version, should always be `6`
        ((u32::from_be(self.header().version_to_flow_label) & 0xf000_0000) >> 28) as u8
    }

    #[inline]
    pub fn dscp(&self) -> u8 {
        ((u32::from_be(self.header().version_to_flow_label) & DSCP) >> 22) as u8
    }

    #[inline]
    pub fn set_dscp(&mut self, dscp: u8) {
        self.header_mut().version_to_flow_label = u32::to_be(
            (u32::from_be(self.header().version_to_flow_label) & !DSCP)
                | ((u32::from(dscp) << 22) & DSCP),
        );
    }

    #[inline]
    pub fn ecn(&self) -> u8 {
        ((u32::from_be(self.header().version_to_flow_label) & ECN) >> 20) as u8
    }

    #[inline]
    pub fn set_ecn(&mut self, ecn: u8) {
        self.header_mut().version_to_flow_label = u32::to_be(
            (u32::from_be(self.header().version_to_flow_label) & !ECN)
                | ((u32::from(ecn) << 20) & ECN),
        );
    }

    #[inline]
    pub fn flow_label(&self) -> u32 {
        u32::from_be(self.header().version_to_flow_label) & FLOW
    }

    #[inline]
    pub fn set_flow_label(&mut self, flow_label: u32) {
        self.header_mut().version_to_flow_label = u32::to_be(
            (u32::from_be(self.header().version_to_flow_label) & !FLOW) | (flow_label & FLOW),
        );
    }

    #[inline]
    pub fn payload_length(&self) -> u16 {
        u16::from_be(self.header().payload_length)
    }

    #[inline]
    fn set_payload_length(&mut self, payload_length: u16) {
        self.header_mut().payload_length = u16::to_be(payload_length);
    }

    #[inline]
    pub fn next_header(&self) -> ProtocolNumber {
        ProtocolNumber::new(self.header().next_header)
    }

    #[inline]
    pub fn set_next_header(&mut self, next_header: ProtocolNumber) {
        self.header_mut().next_header = next_header.0;
    }

    #[inline]
    pub fn hop_limit(&self) -> u8 {
        self.header().hop_limit
    }

    #[inline]
    pub fn set_hop_limit(&mut self, hop_limit: u8) {
        self.header_mut().hop_limit = hop_limit;
    }

    #[inline]
    pub fn src(&self) -> Ipv6Addr {
        self.header().src
    }

    #[inline]
    pub fn set_src(&mut self, src: Ipv6Addr) {
        self.header_mut().src = src;
    }

    #[inline]
    pub fn dst(&self) -> Ipv6Addr {
        self.header().dst
    }

    #[inline]
    pub fn set_dst(&mut self, dst: Ipv6Addr) {
        self.header_mut().dst = dst;
    }
}

impl fmt::Display for Ipv6 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} > {}, version: {}, dscp: {}, ecn: {}, flow_label: {}, len: {}, next_header: {}, hop_limit: {}",
            self.src(),
            self.dst(),
            self.version(),
            self.dscp(),
            self.ecn(),
            self.flow_label(),
            self.payload_len(),
            self.next_header(),
            self.hop_limit()
        )
    }
}

impl Packet for Ipv6 {
    type Header = Ipv6Header;
    type Envelope = Ethernet;

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

        Ok(Ipv6 {
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

        Ok(Ipv6 {
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
        let len = self.payload_len() as u16;
        self.set_payload_length(len);
        self.envelope_mut().cascade();
    }

    #[inline]
    fn deparse(self) -> Self::Envelope {
        self.envelope
    }
}

impl IpPacket for Ipv6 {
    #[inline]
    fn next_proto(&self) -> ProtocolNumber {
        self.next_header()
    }

    #[inline]
    fn src(&self) -> IpAddr {
        IpAddr::V6(self.src())
    }

    #[inline]
    fn set_src(&mut self, src: IpAddr) -> Result<()> {
        match src {
            IpAddr::V6(addr) => {
                self.set_src(addr);
                Ok(())
            }
            _ => Err(IpAddrMismatchError.into()),
        }
    }

    #[inline]
    fn dst(&self) -> IpAddr {
        IpAddr::V6(self.dst())
    }

    #[inline]
    fn set_dst(&mut self, dst: IpAddr) -> Result<()> {
        match dst {
            IpAddr::V6(addr) => {
                self.set_dst(addr);
                Ok(())
            }
            _ => Err(IpAddrMismatchError.into()),
        }
    }

    #[inline]
    fn pseudo_header(&self, packet_len: u16, protocol: ProtocolNumber) -> PseudoHeader {
        PseudoHeader::V6 {
            src: self.src(),
            dst: self.dst(),
            packet_len,
            protocol,
        }
    }
}

impl Ipv6Packet for Ipv6 {}

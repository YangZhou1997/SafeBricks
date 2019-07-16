#![allow(clippy::mut_from_ref)]

use common::Result;
use failure::Fail;
use native::mbuf::MBuf;
use packets::checksum::PseudoHeader;
use packets::ip::v6::Ipv6Packet;
use packets::ip::{IpPacket, ProtocolNumber};
use packets::{buffer, Fixed, Header, Packet, ParseError};
use std::fmt;
use std::net::{IpAddr, Ipv6Addr};

/*  From https://tools.ietf.org/html/draft-ietf-6man-segment-routing-header-16#section-2
    Segment Routing Extension Header (SRH)

    A new type of the Routing Header (originally defined in [RFC8200]) is
    defined: the Segment Routing Header (SRH) which has a new Routing
    Type, (suggested value 4) to be assigned by IANA.

    The Segment Routing Header (SRH) is defined as follows:

     0                   1                   2                   3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    | Next Header   |  Hdr Ext Len  | Routing Type  | Segments Left |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |  Last Entry   |     Flags     |              Tag              |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                                                               |
    |            Segment List[0] (128 bits IPv6 address)            |
    |                                                               |
    |                                                               |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                                                               |
    |                                                               |
                                 ...
    |                                                               |
    |                                                               |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                                                               |
    |            Segment List[n] (128 bits IPv6 address)            |
    |                                                               |
    |                                                               |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    //                                                             //
    //         Optional Type Length Value objects (variable)       //
    //                                                             //
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    where:

    Next Header: 8-bit selector. Identifies the type of header
    immediately following the SRH.

    Hdr Ext Len: 8-bit unsigned integer, is the length of the SRH
    header in 8-octet units, not including the first 8 octets.

    Routing Type: TBD, to be assigned by IANA (suggested value: 4).

    Segments Left: 8-bit unsigned integer Number of route segments
    remaining, i.e., number of explicitly listed intermediate nodes
    still to be visited before reaching the final destination.

    Last Entry: contains the index (zero based), in the Segment List,
    of the last element of the Segment List.

    Flags: 8 bits of flags.  Following flags are defined:

         0 1 2 3 4 5 6 7
        +-+-+-+-+-+-+-+-+
        |U U U U U U U U|
        +-+-+-+-+-+-+-+-+

        U: Unused and for future use.  MUST be 0 on transmission and
        ignored on receipt.

    Tag: tag a packet as part of a class or group of packets, e.g.,
    packets sharing the same set of properties. When tag is not used
    at source it MUST be set to zero on transmission. When tag is not
    used during SRH Processing it SHOULD be ignored. The allocation
    and use of tag is outside the scope of this document.

    Segment List[n]: 128 bit IPv6 addresses representing the nth
    segment in the Segment List.  The Segment List is encoded starting
    from the last segment of the SR Policy.  I.e., the first element
    of the segment list (Segment List [0]) contains the last segment
    of the SR Policy, the second element contains the penultimate
    segment of the SR Policy and so on.

    Type Length Value (TLV) are described in Section 2.1.
*/

/// IPv6 segment routing header
///
/// The segment routing header contains only the fixed portion of the
/// header. `segment_list` and `tlv` are parsed separately.
#[derive(Debug)]
#[repr(C, packed)]
pub struct SegmentRoutingHeader {
    next_header: u8,
    hdr_ext_len: u8,
    routing_type: u8,
    segments_left: u8,
    last_entry: u8,
    flags: u8,
    tag: u16,
}

impl Default for SegmentRoutingHeader {
    fn default() -> SegmentRoutingHeader {
        SegmentRoutingHeader {
            next_header: 0,
            hdr_ext_len: 2,
            routing_type: 4,
            segments_left: 0,
            last_entry: 0,
            flags: 0,
            tag: 0,
        }
    }
}

impl Header for SegmentRoutingHeader {}

/// Type alias for a segment in segment routing header
pub type Segment = Ipv6Addr;

/// Error for a bad segment list
#[derive(Debug, Fail)]
#[fail(display = "Segment list length must be greater than 0")]
pub struct BadSegmentsError;

#[derive(Debug)]
pub struct SegmentRouting<E: Ipv6Packet> {
    envelope: E,
    mbuf: *mut MBuf,
    offset: usize,
    header: *mut SegmentRoutingHeader,
    segments: *mut [Segment],
}

impl<E: Ipv6Packet> SegmentRouting<E> {
    #[inline]
    pub fn next_header(&self) -> ProtocolNumber {
        ProtocolNumber::new(self.header().next_header)
    }

    #[inline]
    pub fn set_next_header(&mut self, next_header: ProtocolNumber) {
        self.header_mut().next_header = next_header.0;
    }

    #[inline]
    pub fn hdr_ext_len(&self) -> u8 {
        self.header().hdr_ext_len
    }

    #[inline]
    fn set_hdr_ext_len(&mut self, hdr_ext_len: u8) {
        self.header_mut().hdr_ext_len = hdr_ext_len;
    }

    #[inline]
    pub fn routing_type(&self) -> u8 {
        self.header().routing_type
    }

    #[inline]
    pub fn segments_left(&self) -> u8 {
        self.header().segments_left
    }

    #[inline]
    pub fn set_segments_left(&mut self, segments_left: u8) {
        self.header_mut().segments_left = segments_left;
    }

    #[inline]
    pub fn last_entry(&self) -> u8 {
        self.header().last_entry
    }

    #[inline]
    fn set_last_entry(&mut self, last_entry: u8) {
        self.header_mut().last_entry = last_entry;
    }

    #[inline]
    pub fn flags(&self) -> u8 {
        self.header().flags
    }

    #[inline]
    pub fn tag(&self) -> u16 {
        u16::from_be(self.header().tag)
    }

    #[inline]
    pub fn set_tag(&mut self, tag: u16) {
        self.header_mut().tag = u16::to_be(tag);
    }

    #[inline]
    pub fn segments(&self) -> &mut [Segment] {
        unsafe { &mut (*self.segments) }
    }

    /// Sets the segment list
    ///
    /// # Examples
    ///
    /// ```
    /// srh.set_segments(&vec![segment1, segment2, segment3, segment4])
    /// ```
    ///
    /// # Remarks
    ///
    /// Be aware that when you call this function, it can affect Tcp and Udp
    /// checksum calculations, as the last segment is used as part of the pseudo
    /// header.
    #[inline]
    pub fn set_segments(&mut self, segments: &[Segment]) -> Result<()> {
        if !segments.is_empty() {
            let old_len = self.last_entry() + 1;
            let new_len = segments.len() as u8;
            let segments_offset = self.offset + SegmentRoutingHeader::size();

            buffer::realloc(
                self.mbuf,
                segments_offset,
                (new_len as isize - old_len as isize) * Segment::size() as isize,
            )?;
            self.segments = buffer::write_slice(self.mbuf, segments_offset, segments)?;
            self.set_hdr_ext_len(new_len * 2);
            self.set_last_entry(new_len - 1);
            Ok(())
        } else {
            Err(BadSegmentsError.into())
        }
    }
}

impl<E: Ipv6Packet> fmt::Display for SegmentRouting<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let segments = self
            .segments()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        write!(
            f,
            "next_header: {}, hdr_ext_len: {}, routing_type: {}, segments_left: {}, last_entry: {}, flags: {}, tag: {}, segments: [{}]",
            self.next_header(),
            self.hdr_ext_len(),
            self.routing_type(),
            self.segments_left(),
            self.last_entry(),
            self.flags(),
            self.tag(),
            segments
        )
    }
}

impl<E: Ipv6Packet> Packet for SegmentRouting<E> {
    type Header = SegmentRoutingHeader;
    type Envelope = E;

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
        Self::Header::size() + self.segments().len() * 16
    }

    #[doc(hidden)]
    #[inline]
    fn do_parse(envelope: Self::Envelope) -> Result<Self> {
        let mbuf = envelope.mbuf();
        let offset = envelope.payload_offset();
        let header = buffer::read_item::<Self::Header>(mbuf, offset)?;
        let hdr_ext_len = unsafe { (*header).hdr_ext_len };
        let segments_len = unsafe { (*header).last_entry + 1 };

        if hdr_ext_len != 0 && (2 * segments_len == hdr_ext_len) {
            let segments = buffer::read_slice::<Segment>(
                mbuf,
                offset + SegmentRoutingHeader::size(),
                segments_len as usize,
            )?;

            Ok(SegmentRouting {
                envelope,
                mbuf,
                offset,
                header,
                segments,
            })
        } else {
            Err(ParseError::new("Packet has inconsistent segment list length").into())
        }
    }

    #[doc(hidden)]
    #[inline]
    fn do_push(envelope: Self::Envelope) -> Result<Self> {
        let mbuf = envelope.mbuf();
        let offset = envelope.payload_offset();

        // also add a default segment list of one element
        buffer::alloc(mbuf, offset, Self::Header::size() + Segment::size())?;
        let header = buffer::write_item::<Self::Header>(mbuf, offset, &Default::default())?;
        let segments =
            buffer::write_slice(mbuf, offset + Self::Header::size(), &[Segment::UNSPECIFIED])?;

        Ok(SegmentRouting {
            envelope,
            mbuf,
            offset,
            header,
            segments,
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

impl<E: Ipv6Packet> IpPacket for SegmentRouting<E> {
    #[inline]
    fn next_proto(&self) -> ProtocolNumber {
        self.next_header()
    }

    #[inline]
    fn src(&self) -> IpAddr {
        self.envelope().src()
    }

    #[inline]
    fn set_src(&mut self, src: IpAddr) -> Result<()> {
        self.envelope_mut().set_src(src)
    }

    #[inline]
    fn dst(&self) -> IpAddr {
        IpAddr::V6(self.segments()[0])
    }

    #[inline]
    fn set_dst(&mut self, dst: IpAddr) -> Result<()> {
        if let IpAddr::V6(v6_dst) = dst {
            let mut segments = vec![v6_dst];
            for segment in self.segments().iter().skip(1) {
                segments.push(*segment)
            }

            self.set_segments(&segments)?;

            if self.segments_left() == 0 {
                self.envelope_mut().set_dst(dst)
            } else {
                Ok(())
            }
        } else {
            unreachable!()
        }
    }

    // From https://tools.ietf.org/html/rfc8200#section-8.1
    //
    // If the IPv6 packet contains a Routing header, the Destination Address
    // used in the pseudo-header is that of the final destination.  At the
    // originating node, that address will be in the last element of the Routing
    // header; at the recipient(s), that address will be in the Destination
    // Address field of the IPv6 header.
    #[inline]
    fn pseudo_header(&self, packet_len: u16, protocol: ProtocolNumber) -> PseudoHeader {
        let dst = match self.dst() {
            IpAddr::V6(dst) => dst,
            _ => unreachable!(),
        };

        let src = match self.src() {
            IpAddr::V6(src) => src,
            _ => unreachable!(),
        };

        PseudoHeader::V6 {
            src,
            dst,
            packet_len,
            protocol,
        }
    }
}

impl<E: Ipv6Packet> Ipv6Packet for SegmentRouting<E> {}

use packets::icmp::v6::{Icmpv6, Icmpv6Packet, Icmpv6Payload, Icmpv6Type, Icmpv6Types, NdpPayload};
use packets::ip::v6::Ipv6Packet;
use std::fmt;

/*  From https://tools.ietf.org/html/rfc4861#section-4.1
    Router Solicitation Message Format

     0                   1                   2                   3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |     Type      |     Code      |          Checksum             |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                            Reserved                           |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |   Options ...
    +-+-+-+-+-+-+-+-+-+-+-+-

    Reserved        This field is unused.  It MUST be initialized to
                    zero by the sender and MUST be ignored by the
                    receiver.

   Valid Options:

    Source link-layer address
                    The link-layer address of the sender, if
                    known.  MUST NOT be included if the Source Address
                    is the unspecified address.  Otherwise, it SHOULD
                    be included on link layers that have addresses.
*/

/// NDP router solicitation message
#[derive(Default, Debug)]
#[repr(C, packed)]
pub struct RouterSolicitation {
    reserved: u32,
}

impl Icmpv6Payload for RouterSolicitation {
    #[inline]
    fn msg_type() -> Icmpv6Type {
        Icmpv6Types::RouterSolicitation
    }
}

impl NdpPayload for RouterSolicitation {}

/// NDP router solicitation packet
impl<E: Ipv6Packet> Icmpv6<E, RouterSolicitation> {
    #[inline]
    pub fn reserved(&self) -> u32 {
        u32::from_be(self.payload().reserved)
    }
}

impl<E: Ipv6Packet> fmt::Display for Icmpv6<E, RouterSolicitation> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "type: {}, code: {}, checksum: 0x{:04x}, reserved: {}",
            self.msg_type(),
            self.code(),
            self.checksum(),
            self.reserved()
        )
    }
}

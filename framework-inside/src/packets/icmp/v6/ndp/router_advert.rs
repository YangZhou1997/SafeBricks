use packets::icmp::v6::{Icmpv6, Icmpv6Packet, Icmpv6Payload, Icmpv6Type, Icmpv6Types, NdpPayload};
use packets::ip::v6::Ipv6Packet;
use std::fmt;

/*  From https://tools.ietf.org/html/rfc4861#section-4.2
    Router Advertisement Message Format

     0                   1                   2                   3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |     Type      |     Code      |          Checksum             |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    | Cur Hop Limit |M|O|  Reserved |       Router Lifetime         |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                         Reachable Time                        |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                          Retrans Timer                        |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |   Options ...
    +-+-+-+-+-+-+-+-+-+-+-+-

    Cur Hop Limit   8-bit unsigned integer.  The default value that
                    should be placed in the Hop Count field of the IP
                    header for outgoing IP packets.  A value of zero
                    means unspecified (by this router).

    M               1-bit "Managed address configuration" flag.  When
                    set, it indicates that addresses are available via
                    Dynamic Host Configuration Protocol [DHCPv6].

                    If the M flag is set, the O flag is redundant and
                    can be ignored because DHCPv6 will return all
                    available configuration information.

    O               1-bit "Other configuration" flag.  When set, it
                    indicates that other configuration information is
                    available via DHCPv6.  Examples of such information
                    are DNS-related information or information on other
                    servers within the network.

      Note: If neither M nor O flags are set, this indicates that no
      information is available via DHCPv6.

    Reserved        A 6-bit unused field.  It MUST be initialized to
                    zero by the sender and MUST be ignored by the
                    receiver.

    Router Lifetime
                    16-bit unsigned integer.  The lifetime associated
                    with the default router in units of seconds.  The
                    field can contain values up to 65535 and receivers
                    should handle any value, while the sending rules in
                    Section 6 limit the lifetime to 9000 seconds.  A
                    Lifetime of 0 indicates that the router is not a
                    default router and SHOULD NOT appear on the default
                    router list.  The Router Lifetime applies only to
                    the router's usefulness as a default router; it
                    does not apply to information contained in other
                    message fields or options.  Options that need time
                    limits for their information include their own
                    lifetime fields.

    Reachable Time  32-bit unsigned integer.  The time, in
                    milliseconds, that a node assumes a neighbor is
                    reachable after having received a reachability
                    confirmation.  Used by the Neighbor Unreachability
                    Detection algorithm (see Section 7.3).  A value of
                    zero means unspecified (by this router).

    Retrans Timer   32-bit unsigned integer.  The time, in
                    milliseconds, between retransmitted Neighbor
                    Solicitation messages.  Used by address resolution
                    and the Neighbor Unreachability Detection algorithm
                    (see Sections 7.2 and 7.3).  A value of zero means
                    unspecified (by this router).

   Possible options:

    Source link-layer address
                    The link-layer address of the interface from which
                    the Router Advertisement is sent.  Only used on
                    link layers that have addresses.  A router MAY omit
                    this option in order to enable inbound load sharing
                    across multiple link-layer addresses.

    MTU             SHOULD be sent on links that have a variable MTU
                    (as specified in the document that describes how to
                    run IP over the particular link type).  MAY be sent
                    on other links.

    Prefix Information
                    These options specify the prefixes that are on-link
                    and/or are used for stateless address
                    autoconfiguration.  A router SHOULD include all its
                    on-link prefixes (except the link-local prefix) so
                    that multihomed hosts have complete prefix
                    information about on-link destinations for the
                    links to which they attach.  If complete
                    information is lacking, a host with multiple
                    interfaces may not be able to choose the correct
                    outgoing interface when sending traffic to its
                    neighbors.
*/

/// NDP router advertisement message
#[derive(Default, Debug)]
#[repr(C, packed)]
pub struct RouterAdvertisement {
    current_hop_limit: u8,
    flags: u8,
    router_lifetime: u16,
    reachable_time: u32,
    retrans_timer: u32,
}

impl Icmpv6Payload for RouterAdvertisement {
    #[inline]
    fn msg_type() -> Icmpv6Type {
        Icmpv6Types::RouterAdvertisement
    }
}

impl NdpPayload for RouterAdvertisement {}

const M_FLAG: u8 = 0b1000_0000;
const O_FLAG: u8 = 0b0100_0000;

/// NDP router advertisement packet
impl<E: Ipv6Packet> Icmpv6<E, RouterAdvertisement> {
    #[inline]
    pub fn current_hop_limit(&self) -> u8 {
        self.payload().current_hop_limit
    }

    #[inline]
    pub fn set_current_hop_limit(&mut self, current_hop_limit: u8) {
        self.payload_mut().current_hop_limit = current_hop_limit;
    }

    #[inline]
    pub fn managed_addr_cfg(&self) -> bool {
        self.payload().flags & M_FLAG != 0
    }

    #[inline]
    pub fn set_managed_addr_cfg(&mut self) {
        self.payload_mut().flags |= M_FLAG;
    }

    #[inline]
    pub fn unset_managed_addr_cfg(&mut self) {
        self.payload_mut().flags &= !M_FLAG;
    }

    #[inline]
    pub fn other_cfg(&self) -> bool {
        self.payload().flags & O_FLAG != 0
    }

    #[inline]
    pub fn set_other_cfg(&mut self) {
        self.payload_mut().flags |= O_FLAG;
    }

    #[inline]
    pub fn unset_other_cfg(&mut self) {
        self.payload_mut().flags &= !O_FLAG;
    }

    #[inline]
    pub fn router_lifetime(&self) -> u16 {
        // TODO: should these times be translated to duration?
        u16::from_be(self.payload().router_lifetime)
    }

    #[inline]
    pub fn set_router_lifetime(&mut self, router_lifetime: u16) {
        self.payload_mut().router_lifetime = u16::to_be(router_lifetime);
    }

    #[inline]
    pub fn reachable_time(&self) -> u32 {
        u32::from_be(self.payload().reachable_time)
    }

    #[inline]
    pub fn set_reachable_time(&mut self, reachable_time: u32) {
        self.payload_mut().reachable_time = u32::to_be(reachable_time);
    }

    #[inline]
    pub fn retrans_timer(&self) -> u32 {
        u32::from_be(self.payload().retrans_timer)
    }

    #[inline]
    pub fn set_retrans_timer(&mut self, retrans_timer: u32) {
        self.payload_mut().retrans_timer = u32::to_be(retrans_timer);
    }
}

impl<E: Ipv6Packet> fmt::Display for Icmpv6<E, RouterAdvertisement> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "type: {}, code: {}, checksum: 0x{:04x}, current_hop_limit: {}, managed address cfg: {}, other cfg: {}, router_lifetime: {}, reachable_time: {}, retrans_timer: {}",
            self.msg_type(),
            self.code(),
            self.checksum(),
            self.current_hop_limit(),
            self.managed_addr_cfg(),
            self.other_cfg(),
            self.router_lifetime(),
            self.reachable_time(),
            self.retrans_timer()
        )
    }
}

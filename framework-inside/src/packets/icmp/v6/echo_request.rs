use common::Result;
use packets::icmp::v6::{Icmpv6, Icmpv6Packet, Icmpv6Payload, Icmpv6Type, Icmpv6Types};
use packets::ip::v6::Ipv6Packet;
use packets::{buffer, Fixed, Packet};
use std::fmt;

/*  From https://tools.ietf.org/html/rfc4443#section-4.1
    Echo Request Message

     0                   1                   2                   3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |     Type      |     Code      |          Checksum             |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |           Identifier          |        Sequence Number        |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |     Data ...
    +-+-+-+-+-

    Identifier      An identifier to aid in matching Echo Replies
                    to this Echo Request.  May be zero.

    Sequence Number
                    A sequence number to aid in matching Echo Replies
                    to this Echo Request.  May be zero.

    Data            Zero or more octets of arbitrary data.
*/

/// Echo request message
#[derive(Default, Debug)]
#[repr(C, packed)]
pub struct EchoRequest {
    identifier: u16,
    seq_no: u16,
}

impl Icmpv6Payload for EchoRequest {
    fn msg_type() -> Icmpv6Type {
        Icmpv6Types::EchoRequest
    }
}

impl<E: Ipv6Packet> Icmpv6<E, EchoRequest> {
    #[inline]
    pub fn identifier(&self) -> u16 {
        u16::from_be(self.payload().identifier)
    }

    #[inline]
    pub fn set_identifier(&mut self, identifier: u16) {
        self.payload_mut().identifier = u16::to_be(identifier);
    }

    #[inline]
    pub fn seq_no(&self) -> u16 {
        u16::from_be(self.payload().seq_no)
    }

    #[inline]
    pub fn set_seq_no(&mut self, seq_no: u16) {
        self.payload_mut().seq_no = u16::to_be(seq_no);
    }

    /// Returns the offset where the data field in the message body starts
    #[inline]
    fn data_offset(&self) -> usize {
        self.payload_offset() + EchoRequest::size()
    }

    /// Returns the length of the data field in the message body
    #[inline]
    fn data_len(&self) -> usize {
        self.payload_len() - EchoRequest::size()
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        if let Ok(data) = buffer::read_slice(self.mbuf(), self.data_offset(), self.data_len()) {
            unsafe { &(*data) }
        } else {
            unreachable!()
        }
    }

    #[inline]
    pub fn set_data(&mut self, data: &[u8]) -> Result<()> {
        buffer::realloc(
            self.mbuf(),
            self.data_offset(),
            data.len() as isize - self.data_len() as isize,
        )?;
        buffer::write_slice(self.mbuf(), self.data_offset(), data)?;
        Ok(())
    }
}

impl<E: Ipv6Packet> fmt::Display for Icmpv6<E, EchoRequest> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "type: {}, code: {}, checksum: 0x{:04x}, identifier: {}, seq_no: {}",
            self.msg_type(),
            self.code(),
            self.checksum(),
            self.identifier(),
            self.seq_no(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_of_echo_request() {
        assert_eq!(4, EchoRequest::size());
    }
}

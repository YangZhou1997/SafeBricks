use fallible_iterator::FallibleIterator;
use native::mbuf::MBuf;
use packets::{buffer, ParseError};

pub use self::link_layer_addr::*;
pub use self::mtu::*;
pub use self::prefix_info::*;

pub mod link_layer_addr;
pub mod mtu;
pub mod prefix_info;

const SOURCE_LINK_LAYER_ADDR: u8 = 1;
const TARGET_LINK_LAYER_ADDR: u8 = 2;
const PREFIX_INFORMATION: u8 = 3;
//const REDIRECTED_HEADER: u8 = 4;
const MTU: u8 = 5;

/// A parsed NDP option
pub enum NdpOption {
    SourceLinkLayerAddress(LinkLayerAddress),
    TargetLinkLayerAddress(LinkLayerAddress),
    PrefixInformation(PrefixInformation),
    Mtu(Mtu),
    /// An undefined NDP option
    Undefined(u8, u8),
}

/// NDP options iterator
pub struct NdpOptionsIterator {
    mbuf: *mut MBuf,
    offset: usize,
}

impl NdpOptionsIterator {
    pub fn new(mbuf: *mut MBuf, offset: usize) -> NdpOptionsIterator {
        NdpOptionsIterator { mbuf, offset }
    }
}

impl FallibleIterator for NdpOptionsIterator {
    type Item = NdpOption;
    type Error = failure::Error;

    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        let buffer_len = unsafe { (*self.mbuf).data_len() };

        if self.offset <= buffer_len {
            let [option_type, length] =
                unsafe { *(buffer::read_item::<[u8; 2]>(self.mbuf, self.offset)?) };

            if length == 0 {
                Err(ParseError::new("NDP option has zero length").into())
            } else {
                let option = match option_type {
                    SOURCE_LINK_LAYER_ADDR => {
                        let option = LinkLayerAddress::parse(self.mbuf, self.offset)?;
                        NdpOption::SourceLinkLayerAddress(option)
                    }
                    TARGET_LINK_LAYER_ADDR => {
                        let option = LinkLayerAddress::parse(self.mbuf, self.offset)?;
                        NdpOption::TargetLinkLayerAddress(option)
                    }
                    PREFIX_INFORMATION => {
                        let option = PrefixInformation::parse(self.mbuf, self.offset)?;
                        NdpOption::PrefixInformation(option)
                    }
                    MTU => {
                        let option = Mtu::parse(self.mbuf, self.offset)?;
                        NdpOption::Mtu(option)
                    }
                    _ => NdpOption::Undefined(option_type, length),
                };

                self.offset += (length * 8) as usize;
                Ok(Some(option))
            }
        } else {
            Ok(None)
        }
    }
}

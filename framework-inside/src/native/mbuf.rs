
pub const MAX_MBUF_SIZE: u16 = 2048;

/* 
In our simulated MBuf: 
    pkt_len = data_len: is the length of the ethernet packet. 
    buf_len = MAX_MBUF_SIZE;
    data_off = 0: is the packet starting address in buf_addr
    buf_addr array stores the ethernet packet data.
*/
pub struct MBuf{
    data_off: u16,
    buf_len: u16,
    data_len: u16,
    pkt_len: u32,
    buf_addr: [u8; MAX_MBUF_SIZE as usize],
}


impl MBuf {

    #[inline]
    pub fn new(pkt_len: u32) -> MBuf {
        assert!(pkt_len <= (MAX_MBUF_SIZE as u32));
        MBuf{
            data_off: 0, 
            buf_len: MAX_MBUF_SIZE,
            data_len: pkt_len as u16, 
            pkt_len, 
            buf_addr: [0; MAX_MBUF_SIZE as usize],
        }
    }

    #[inline]
    pub fn data_address(&mut self, offset: usize) -> *mut u8 {
        unsafe { (&mut (self.buf_addr[0]) as *mut u8).offset(self.data_off as isize + offset as isize) }
    }

    /// Returns the total allocated size of this mbuf segment.
    /// This is a constant.
    #[inline]
    pub fn buf_len(&self) -> usize {
        self.buf_len as usize
    }

    /// Returns the length of data in this mbuf segment.
    #[inline]
    pub fn data_len(&self) -> usize {
        self.data_len as usize
    }

    /// Returns the size of the packet (across multiple mbuf segment).
    #[inline]
    pub fn pkt_len(&self) -> usize {
        self.pkt_len as usize
    }

    #[inline]
    fn pkt_headroom(&self) -> usize {
        self.data_off as usize
    }

    #[inline]
    fn pkt_tailroom(&self) -> usize {
        self.buf_len() - self.data_off as usize - self.data_len()
    }

    /// Add data to the beginning of the packet. This might fail (i.e., return 0) when no more headroom is left.
    #[inline]
    pub fn add_data_beginning(&mut self, len: usize) -> usize {
        // If only we could add a likely here.
        if len > self.pkt_headroom() {
            0
        } else {
            self.data_off -= len as u16;
            self.data_len += len as u16;
            self.pkt_len += len as u32;
            len
        }
    }

    /// Add data to the end of a packet buffer. This might fail (i.e., return 0) when no more tailroom is left. We do
    /// not currently deal with packet with multiple segments.
    #[inline]
    pub fn add_data_end(&mut self, len: usize) -> usize {
        if len > self.pkt_tailroom() {
            0
        } else {
            self.data_len += len as u16;
            self.pkt_len += len as u32;
            len
        }
    }

    #[inline]
    pub fn remove_data_beginning(&mut self, len: usize) -> usize {
        if len > self.data_len() {
            0
        } else {
            self.data_off += len as u16;
            self.data_len -= len as u16;
            self.pkt_len -= len as u32;
            len
        }
    }

    #[inline]
    pub fn remove_data_end(&mut self, len: usize) -> usize {
        if len > self.data_len() {
            0
        } else {
            self.data_len -= len as u16;
            self.pkt_len -= len as u32;
            len
        }
    }
}

use super::super::native_include as ldpdk;
// use self::ldpdk::*;
pub type MBuf = ldpdk::rte_mbuf;
pub const MAX_MBUF_SIZE: u16 = 2048;

impl Drop for MBuf {
    fn drop(&mut self) {
        unsafe {
            println!("We do not allow MBuf to be freed by Rust (dpdk will free it)");
        }
    }
}

impl MBuf {
    #[inline]
    pub fn read_metadata_slot(mbuf: *mut MBuf, slot: usize) -> usize {
        unsafe {
            let ptr = (mbuf.offset(1) as *mut usize).add(slot);
            *ptr
        }
    }

    // #[inline]
    // pub fn new(pkt_len: u32) -> MBuf {
    //     assert!(pkt_len <= (MAX_MBUF_SIZE as u32));
    //     MBuf{
    //         cacheline0: MARKER,
    //         buf_addr: *mut ::std::os::raw::c_void,
    //         buf_physaddr: Default::default(),
    //         rearm_data: MARKER64,
    //         data_off: Default::default(),
    //         __bindgen_anon_1: rte_mbuf__bindgen_ty_1{0},
    //         nb_segs: Default::default(),
    //         port: Default::default(),
    //         ol_flags: Default::default(),
    //         rx_descriptor_fields1: MARKER,
    //         __bindgen_anon_2: rte_mbuf__bindgen_ty_2,
    //         pkt_len: Default::default(),
    //         data_len: Default::default(),
    //         vlan_tci: Default::default(),
    //         hash: rte_mbuf__bindgen_ty_3,
    //         vlan_tci_outer: Default::default(),
    //         buf_len: Default::default(),
    //         timestamp: Default::default(),
    //         cacheline1: MARKER,
    //         __bindgen_anon_3: rte_mbuf__bindgen_ty_4,
    //         pool: *mut rte_mempool,
    //         next: *mut rte_mbuf,
    //         __bindgen_anon_4: rte_mbuf__bindgen_ty_5,
    //         priv_size: Default::default(),
    //         timesync: Default::default(),
    //         seqn: Default::default(),
    //         __bindgen_padding_0: Default::default(),
    //     }
    // }

    #[inline]
    pub fn write_metadata_slot(mbuf: *mut MBuf, slot: usize, value: usize) {
        unsafe {
            let ptr = (mbuf.offset(1) as *mut usize).add(slot);
            *ptr = value;
        }
    }

    #[inline]
    pub unsafe fn metadata_as<T: Sized>(mbuf: *const MBuf, slot: usize) -> *const T {
        (mbuf.offset(1) as *const usize).add(slot) as *const T
    }

    #[inline]
    pub unsafe fn mut_metadata_as<T: Sized>(mbuf: *mut MBuf, slot: usize) -> *mut T {
        (mbuf.offset(1) as *mut usize).add(slot) as *mut T
    }

    #[inline]
    pub fn data_address(&self, offset: usize) -> *mut u8 {
        unsafe { (self.buf_addr as *mut u8).offset(self.data_off as isize + offset as isize) }
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

    #[inline]
    pub fn refcnt(&self) -> u16 {
        unsafe { self.__bindgen_anon_1.refcnt }
    }

    #[inline]
    pub fn reference(&mut self) {
        unsafe {
            self.__bindgen_anon_1.refcnt += 1;
        }
    }
}

use common::*;
use failure::Fail;
use std::cmp::min;
use std::io::{Read, Write};

/// Shareable data structures.
use std::io::Error;
use std::ptr;
use utils::PAGE_SIZE;
use std::slice;
use native::mbuf::MBuf;
use std::sync::atomic::compiler_fence;
use std::sync::atomic::Ordering;

pub const sendq_name: &str = "safebricks_sendq";
pub const recvq_name: &str = "safebricks_recvq";
pub const mbufq_name: &str = "safebricks_mbufq";

/// Error related to the RingBuffer
#[derive(Debug, Fail)]
#[fail(display = "Bad ring size {}, must be a power of 2", _0)]
struct InvalidRingSize(usize);

/// A ring buffer which can be used to insert and read ordered data.
pub struct RingBuffer {
    /// boxed ring; avoid heap memory being dropped;
    boxed: Box<[u8]>, 
    /// Head, signifies where a consumer should read from.
    pub head: *mut usize,
    /// Tail, signifies where a producer should write.
    pub tail: *mut usize,
    /// Size of the ring buffer.
    size: *mut usize,
    /// Mask used for bit-wise wrapping operations.
    mask: *mut usize,
    /// A Vec that holds this RingBuffer's data.
    vec: *mut (*mut MBuf),
}

unsafe impl Send for RingBuffer {}

#[cfg_attr(feature = "dev", allow(len_without_is_empty))]
impl RingBuffer {
    /// Create a new wrapping ring buffer. The ring buffer size is specified in bytes and must be a power of 2. 
    /// bytes is the number of bytes of RingBuffer::vec
    /// we will require additional 16 bytes to store the meta-data for this ring.
    pub unsafe fn new_in_heap(ring_size: usize) -> Result<RingBuffer>{
        if ring_size & (ring_size - 1) != 0 {
            // We need pages to be a power of 2.
            return Err(InvalidRingSize(ring_size).into());
        }

        let temp_vec: Vec<u8> = vec![0; ring_size * 8 + 16];
        let mut boxed: Box<[u8]> = temp_vec.into_boxed_slice(); // Box<[u8]> is just like &[u8];

        let address = &mut boxed[0] as *mut u8;
        unsafe{
            *(address as *mut usize) = 0;
            *((address as *mut usize).offset(1)) = 0;
            *((address as *mut usize).offset(2)) = ring_size;
            *((address as *mut usize).offset(3)) = ring_size - 1;
        }

        Ok(RingBuffer {
            boxed,
            head: (address as *mut usize),
            tail: (address as *mut usize).offset(1), 
            size: (address as *mut usize).offset(2),
            mask: (address as *mut usize).offset(3),
            vec: (address as *mut usize).offset(4) as (*mut (*mut MBuf)),
        })
    }


    #[inline]
    pub fn head(&self) -> usize{
        unsafe{(*self.head)}
    }
    #[inline]
    pub fn set_head(&self, new_head: usize){
        unsafe{*self.head = new_head;}
    }
    #[inline]
    pub fn wrapping_sub_head(&self, delta: usize)
    {
        self.set_head(self.head().wrapping_sub(delta));        
    }
    #[inline]
    pub fn wrapping_add_head(&self, delta: usize)
    {
        self.set_head(self.head().wrapping_add(delta));        
    }

    #[inline]
    pub fn tail(&self) -> usize{
        unsafe{(*self.tail)}
    }
    #[inline]
    pub fn set_tail(&self, new_tail: usize){
        unsafe{*self.tail = new_tail;}
    }
    #[inline]
    pub fn wrapping_sub_tail(&self, delta: usize)
    {
        self.set_tail(self.tail().wrapping_sub(delta));        
    }
    #[inline]
    pub fn wrapping_add_tail(&self, delta: usize)
    {
        self.set_tail(self.tail().wrapping_add(delta));
    }

    #[inline]
    pub fn size(&self) -> usize{
        unsafe{(*self.size)}
    }
    #[inline]
    pub fn set_size(&self, new_size: usize){
        unsafe{*self.size = new_size;}
    }

    #[inline]
    pub fn mask(&self) -> usize{
        unsafe{(*self.mask)}
    }
    #[inline]
    pub fn set_mask(&self, new_mask: usize){
        unsafe{*self.mask = new_mask;}
    }
    
    /// Read from the buffer, incrementing the read head. Returns bytes read.
    #[inline]
    pub fn read_from_head(&self, mbufs: &mut [*mut MBuf]) -> usize {
        let available = self.tail().wrapping_sub(self.head());
        let to_read = min(mbufs.len(), available);
        let offset = self.head() & self.mask();
        let reads = self.wrapped_read(offset, &mut mbufs[..to_read]);
        compiler_fence(Ordering::Release);
        self.wrapping_add_head(reads);
        reads
    }

    /// Write data at the end of the buffer. The amount of data written might be smaller than input.
    #[inline]
    pub fn write_at_tail(&self, mbufs: &[*mut MBuf]) -> usize {
        let available = self.size().wrapping_add(self.head()).wrapping_sub(self.tail());
        let to_write = min(mbufs.len(), available);
        let offset = self.tail() & self.mask();
        let writes = self.wrapped_write(offset, &mbufs[..to_write]);
        compiler_fence(Ordering::Release);
        self.wrapping_add_tail(writes);
        writes
    }

    /// Reads data from self.vec, wrapping around the end of the Vec if necessary. Returns the
    /// number of bytes written.
    fn wrapped_read(&self, offset: usize, mbufs: &mut [*mut MBuf]) -> usize {
        let mut bytes: usize = 0;
        let ring_size = self.size();
        assert!(offset < ring_size);
        assert!(mbufs.len() <= ring_size);

        let mut bytes = min(ring_size - offset, mbufs.len());
        if bytes != 0 {
            unsafe{ ptr::copy(self.vec.offset(offset as isize), &mut mbufs[0] as (*mut (*mut MBuf)), bytes) };
        }
        if offset + mbufs.len() > ring_size {
            let remaining = mbufs.len() - bytes;
            unsafe{ ptr::copy(self.vec, ((&mut mbufs[0]) as (*mut (*mut MBuf))).offset(bytes as isize), remaining) };
            bytes += remaining;
        }
        bytes
    }

    /// Writes data to self.vec[offset..], wrapping around the end of the Vec if necessary. Returns
    /// the number of bytes written.
    fn wrapped_write(&self, offset: usize, mbufs: &[*mut MBuf]) -> usize {
        let mut bytes: usize = 0;
        let ring_size = self.size();
        assert!(offset < ring_size);
        assert!(mbufs.len() <= ring_size);

        let mut bytes = min(ring_size - offset, mbufs.len());
        if bytes != 0 {
            unsafe{ ptr::copy(&mbufs[0] as (*const (* mut MBuf)), self.vec.offset(offset as isize), bytes) };
        }
        if offset + mbufs.len() > ring_size {
            let remaining = mbufs.len() - bytes;
            unsafe{ ptr::copy(((&mbufs[0]) as (*const (* mut MBuf))).offset(bytes as isize), self.vec, remaining) };
            bytes += remaining;
        }
        bytes
    }

    /// Length of the ring buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.size()
    }

    /// If the ring buffer is empty or not.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn clear(&self) {
        self.set_head(0);
        self.set_tail(0);
    }
}

use common::*;
use failure::Fail;
use std::cmp::min;
use std::io::{Read, Write};

/// Shareable data structures.
use libc::{self, c_void, close, ftruncate, mmap, munmap, shm_open, shm_unlink};
use std::ffi::CString;
use std::io::Error;
use std::ptr;
use utils::PAGE_SIZE;
use std::slice;

pub const sendq_name: &str = "safebricks_sendq";
pub const recvq_name: &str = "safebricks_recvq";
pub const mbufq_name: &str = "safebricks_mbufq";

/// Error related to the RingBuffer
#[derive(Debug, Fail)]
#[fail(display = "Bad ring size {}, must be a power of 2", _0)]
struct InvalidRingSize(usize);

/// A ring buffer which can be used to insert and read ordered data.
pub struct RingBuffer {
    /// the shared memory region name; note this string is not stored in the shared memory region.
    name: CString,
    /// Head, signifies where a consumer should read from.
    head: *mut usize,
    /// Tail, signifies where a producer should write.
    tail: *mut usize,
    /// Size of the ring buffer.
    size: *mut usize,
    /// Mask used for bit-wise wrapping operations.
    mask: *mut usize,
    /// A Vec that holds this RingBuffer's data.
    vec: *mut u8,
}


impl Drop for RingBuffer {
    fn drop(&mut self) {
        unsafe {
            let size = self.size() + 16; // plus the four usize meta-value stored in shared memory. 
            let _ret = munmap(self.vec as *mut c_void, size); // Unmap pages.
                                                              // Record munmap failure.
            let shm_ret = shm_unlink(self.name.as_ptr());
            assert!(shm_ret == 0, "Could not unlink shared memory region");
        }
    }
}

unsafe impl Send for RingBuffer {}

#[cfg_attr(feature = "dev", allow(len_without_is_empty))]
impl RingBuffer {
    /// Create a new wrapping ring buffer. The ring buffer size is specified in bytes and must be a power of 2. 
    /// bytes is the number of bytes of RingBuffer::vec
    pub unsafe fn new_in_shared_mem(name: &str, bytes: usize) -> Result<RingBuffer>{
        if bytes & (bytes - 1) != 0 {
            // We need pages to be a power of 2.
            return Err(InvalidRingSize(bytes).into());
        }
        // // Make sure size is page aligned
        // assert!(bytes & !PAGE_SIZE == 0);

        let name = CString::new(name).unwrap();
        let mut fd = shm_open(
            name.as_ptr(),
            libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
            0o700,
        );
        if fd == -1 {
            if let Some(e) = Error::last_os_error().raw_os_error() {
                if e == libc::EEXIST {
                    shm_unlink(name.as_ptr());
                    fd = shm_open(
                        name.as_ptr(),
                        libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                        0o700,
                    );
                }
            }
        };
        assert!(fd >= 0, "Could not create shared memory segment");
        let ftret = ftruncate(fd, bytes as i64);
        assert!(ftret == 0, "Could not truncate");
        let address = mmap(
            ptr::null_mut(),
            bytes + 16,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_POPULATE | libc::MAP_PRIVATE,
            fd,
            0,
        );
        if address == libc::MAP_FAILED {
            let err_string = CString::new("mmap failed").unwrap();
            libc::perror(err_string.as_ptr());
            panic!("Could not mmap shared region");
        }
        close(fd);

        unsafe{
            *(address as *mut usize) = 0;
            *((address as *mut usize).offset(1)) = 0;
            *((address as *mut usize).offset(2)) = bytes;
            *((address as *mut usize).offset(3)) = bytes - 1;
        }

        Ok(RingBuffer {
            name,
            head: (address as *mut usize),
            tail: (address as *mut usize).offset(1), 
            size: (address as *mut usize).offset(2),
            mask: (address as *mut usize).offset(3),
            vec: (address as *mut usize).offset(4) as *mut u8,
        })
    }


    #[inline]
    fn head(&self) -> usize{
        unsafe{(*self.head)}
    }
    #[inline]
    fn set_head(&mut self, new_head: usize){
        unsafe{*self.head = new_head;}
    }
    #[inline]
    fn wrapping_sub_head(&mut self, delta: usize)
    {
        unsafe{(*self.head).wrapping_sub(delta);}
    }
    #[inline]
    fn wrapping_add_head(&mut self, delta: usize)
    {
        unsafe{(*self.head).wrapping_add(delta);}
    }

    #[inline]
    fn tail(&self) -> usize{
        unsafe{(*self.tail)}
    }
    #[inline]
    fn set_tail(&mut self, new_tail: usize){
        unsafe{*self.tail = new_tail;}
    }
    #[inline]
    fn wrapping_sub_tail(&mut self, delta: usize)
    {
        unsafe{(*self.tail).wrapping_sub(delta);}
    }
    #[inline]
    fn wrapping_add_tail(&mut self, delta: usize)
    {
        unsafe{(*self.tail).wrapping_add(delta);}
    }

    #[inline]
    fn size(&self) -> usize{
        unsafe{(*self.size)}
    }
    #[inline]
    fn set_size(&mut self, new_size: usize){
        unsafe{*self.size = new_size;}
    }

    #[inline]
    fn mask(&self) -> usize{
        unsafe{(*self.mask)}
    }
    #[inline]
    fn set_mask(&mut self, new_mask: usize){
        unsafe{*self.mask = new_mask;}
    }
    
    #[inline]
    fn vec_as_u8(&self) -> &[u8]{
        unsafe{slice::from_raw_parts(self.vec as *const u8, self.size())}
    }
    #[inline]
    fn vec_as_mut_u8(&mut self) -> &mut [u8]{
        unsafe{slice::from_raw_parts_mut(self.vec, self.size())}
    }


    /// Read from the buffer, incrementing the read head. Returns bytes read.
    #[inline]
    pub fn read_from_head(&mut self, data: &mut [u8]) -> usize {
        let len = data.len();
        self.read_from_head_with_increment(data, len)
    }

    /// Write data at the end of the buffer. The amount of data written might be smaller than input.
    #[inline]
    pub fn write_at_tail(&mut self, data: &[u8]) -> usize {
        let available = self.mask().wrapping_add(self.head()).wrapping_sub(self.tail());
        let write = min(data.len(), available);
        if write != data.len() {
            info!("Not writing all, available {}", available);
        }
        let offset = self.tail() & self.mask();
        self.wrapping_add_tail(write);
        self.wrapped_write(offset, &data[..write])
    }

    /// Reads data from self.vec, wrapping around the end of the Vec if necessary. Returns the
    /// number of bytes written.
    fn wrapped_read(&mut self, offset: usize, data: &mut [u8]) -> usize {
        let mut bytes: usize = 0;
        let ring_size = self.size();
        assert!(offset < ring_size);
        assert!(data.len() <= ring_size);

        let u8_vec: &[u8]= self.vec_as_u8();
        bytes += (&u8_vec[offset..]).read(data).unwrap();
        if offset + data.len() > ring_size {
            let remaining = data.len() - bytes;
            bytes += (&u8_vec[..remaining]).read(&mut data[bytes..]).unwrap();
        }
        bytes
    }

    /// Writes data to self.vec[offset..], wrapping around the end of the Vec if necessary. Returns
    /// the number of bytes written.
    fn wrapped_write(&mut self, offset: usize, data: &[u8]) -> usize {
        let mut bytes: usize = 0;
        let ring_size = self.size();
        assert!(offset < ring_size);
        assert!(data.len() <= ring_size);

        let mut_u8_vec: &mut [u8]= self.vec_as_mut_u8();
        bytes += (&mut mut_u8_vec[offset..]).write(data).unwrap();
        if offset + data.len() > ring_size {
            let remaining = data.len() - bytes;
            bytes += (&mut mut_u8_vec[..remaining]).write(&data[bytes..]).unwrap();
        }
        bytes
    }

    /// Data available to be read.
    #[inline]
    pub fn available(&self) -> usize {
        self.tail().wrapping_sub(self.head())
    }

    #[inline]
    fn read_offset(&self) -> usize {
        self.head() & self.mask()
    }

    /// Read from the buffer, incrementing the read head by `increment` bytes. Returns bytes read.
    #[inline]
    pub fn read_from_head_with_increment(&mut self, data: &mut [u8], increment: usize) -> usize {
        let offset = self.read_offset();
        let to_read = min(self.available(), data.len());
        self.wrapping_add_head(min(increment, to_read));        
        self.wrapped_read(offset, &mut data[..to_read])
    }

    /// Seek the read head by `seek` bytes (without actually reading any data). `seek` must be less-than-or-equal to the
    /// number of available bytes.
    #[inline]
    pub fn seek_head(&mut self, seek: usize) {
        let available = self.available();
        assert!(available >= seek, "Seek beyond available bytes.");
        self.wrapping_add_head(seek);
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

    /// In cases with out-of-order data this allows the write head (and hence the amount of available data) to be
    /// progressed without writing anything.
    #[inline]
    pub fn seek_tail(&mut self, increment_by: usize) {
        self.tail = self.tail.wrapping_add(increment_by);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.set_head(0);
        self.set_tail(0);
    }
}

use std::cmp::min;
use std::io::{Read, Write};
use std::io::Error as IOError;
use std::ptr;
use std::slice;
use std::sync::atomic::compiler_fence;
use std::sync::atomic::Ordering;
use std::ffi::CString;
use std::result::Result;

use failure::Fail;
use failure::Error;
use libc::{self, c_void, close, ftruncate, mmap, munmap, shm_open, shm_unlink};

pub const SENDQ_PREFIX: &str = "sb_sendq";
pub const RECVQ_PREFIX: &str = "sb_recvq";

/// Error related to the RingBuffer
#[derive(Debug, Fail)]
#[fail(display = "Bad ring size {}, must be a power of 2", _0)]
struct InvalidRingSize(usize);

struct SuperBox { my_box: Box<[u8]> }

impl Drop for SuperBox {
    fn drop(&mut self) {
        unsafe {
            println!("SuperBox freed");
        }
    }
}

// (*mut MBuf) -> u64
#[derive(Clone)]
struct SuperVec { my_vec: *mut u64 }

impl Drop for SuperVec {
    fn drop(&mut self) {
        unsafe {
            println!("SuperVec freed");
        }
    }
}


#[derive(Clone)]
pub struct SuperUsize { pub my_usize: *mut usize }

impl Drop for SuperUsize {
    fn drop(&mut self) {
        unsafe {
            println!("SuperUsize freed");
        }
    }
}
pub const STOP_MARK: u32 = 0xabcdefff;

/// A ring buffer which can be used to insert and read ordered data.
pub struct RingBuffer {
    /// boxed ring; avoid heap memory being dropped;
    // boxed: SuperBox,
    /// RingBuffer name
    pub name: CString,
    /// The size of the shared memory region;
    pub size_shm: usize,
    /// The pointer to the shared memory region;
    pub mem: *mut u8,
    /// 
    /// Head, signifies where a consumer should read from.
    pub head: SuperUsize,
    /// Tail, signifies where a producer should write.
    pub tail: SuperUsize,
    /// Size of the ring buffer.
    pub size: SuperUsize,
    /// Mask used for bit-wise wrapping operations.
    pub mask: SuperUsize,
    /// A Vec that holds this RingBuffer's data.
    vec: SuperVec,
}

// only outside ring buffer has the drop function. 
impl Drop for RingBuffer {
    fn drop(&mut self) {
        unsafe {
            unsafe {
                let size = self.size_shm;
                let _ret = munmap(self.mem as *mut c_void, size); // Unmap pages.
                                                                // Record munmap failure.
                let shm_ret = shm_unlink(self.name.as_ptr());
                assert!(shm_ret == 0, "Could not unlink shared memory region");
            }
            println!("RingBuffer freed");
        }
    }
}
unsafe impl Send for RingBuffer {}

#[cfg_attr(feature = "dev", allow(len_without_is_empty))]
impl RingBuffer {
    /// Create/attach a new wrapping ring buffer. 
    /// The ring buffer size is specified in bytes and must be a power of 2. 
    /// we will require additional 16 bytes to store the meta-data for this ring.
    pub unsafe fn new_in_heap(ring_size: usize, name: &str) -> Result<RingBuffer, Error>{
        if ring_size & (ring_size - 1) != 0 {
            // We need pages to be a power of 2.
            return Err(InvalidRingSize(ring_size).into());
        }
        let size = ring_size * 8 + 16;

        // let temp_vec: Vec<u8> = vec![0; ring_size * 8 + 16];
        // let mut boxed: SuperBox = SuperBox{ my_box: temp_vec.into_boxed_slice(), }; // Box<[u8]> is just like &[u8];
        let name = CString::new(name).unwrap();
        let mut fd = shm_open(
            name.as_ptr(),
            libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
            0o700,
        );
        if fd == -1 {
            if let Some(e) = IOError::last_os_error().raw_os_error() {
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
        let ftret = ftruncate(fd, size as i64);
        assert!(ftret == 0, "Could not truncate");
        let address = mmap(
            ptr::null_mut(),
            size,
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

        let address = address as *mut u8;
        // let address = &mut boxed.my_box[0] as *mut u8;
        unsafe{
            *(address as *mut usize) = 0;
            *((address as *mut usize).offset(1)) = 0;
            *((address as *mut usize).offset(2)) = ring_size;
            *((address as *mut usize).offset(3)) = ring_size - 1;
        }

        Ok(RingBuffer {
            name, 
            size_shm: size, 
            mem: address, 
            head: SuperUsize{ my_usize: (address as *mut usize) },
            tail: SuperUsize{ my_usize: (address as *mut usize).offset(1) },
            size: SuperUsize{ my_usize: (address as *mut usize).offset(2) },
            mask: SuperUsize{ my_usize: (address as *mut usize).offset(3) },
            vec: SuperVec{ my_vec: (address as *mut usize).offset(4) as (*mut u64) },
        })
    }

    #[inline]
    pub fn head(&self) -> usize{
        unsafe{(*self.head.my_usize)}
    }
    #[inline]
    pub fn set_head(&self, new_head: usize){
        unsafe{*self.head.my_usize = new_head;}
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
        unsafe{(*self.tail.my_usize)}
    }
    #[inline]
    pub fn set_tail(&self, new_tail: usize){
        unsafe{*self.tail.my_usize = new_tail;}
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
        unsafe{(*self.size.my_usize)}
    }
    #[inline]
    pub fn set_size(&self, new_size: usize){
        unsafe{*self.size.my_usize = new_size;}
    }

    #[inline]
    pub fn mask(&self) -> usize{
        unsafe{(*self.mask.my_usize)}
    }
    #[inline]
    pub fn set_mask(&self, new_mask: usize){
        unsafe{*self.mask.my_usize = new_mask;}
    }
    
    /// Read from the buffer, incrementing the read head. Returns bytes read.
    #[inline]
    pub fn read_from_head(&self, mbufs: &mut [u64]) -> usize {
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
    pub fn write_at_tail(&self, mbufs: &[u64]) -> usize {
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
    fn wrapped_read(&self, offset: usize, mbufs: &mut [u64]) -> usize {
        let mut bytes: usize = 0;
        let ring_size = self.size();
        assert!(offset < ring_size);
        assert!(mbufs.len() <= ring_size);

        let mut bytes = min(ring_size - offset, mbufs.len());
        if bytes != 0 {
            unsafe{ ptr::copy(self.vec.my_vec.offset(offset as isize), &mut mbufs[0] as (*mut u64), bytes) };
        }
        if offset + mbufs.len() > ring_size {
            let remaining = mbufs.len() - bytes;
            unsafe{ ptr::copy(self.vec.my_vec, ((&mut mbufs[0]) as (*mut u64)).offset(bytes as isize), remaining) };
            bytes += remaining;
        }
        bytes
    }

    /// Writes data to self.vec[offset..], wrapping around the end of the Vec if necessary. Returns
    /// the number of bytes written.
    fn wrapped_write(&self, offset: usize, mbufs: &[u64]) -> usize {
        let mut bytes: usize = 0;
        let ring_size = self.size();
        assert!(offset < ring_size);
        assert!(mbufs.len() <= ring_size);

        let mut bytes = min(ring_size - offset, mbufs.len());
        if bytes != 0 {
            unsafe{ ptr::copy(&mbufs[0] as (*const u64), self.vec.my_vec.offset(offset as isize), bytes) };
        }
        if offset + mbufs.len() > ring_size {
            let remaining = mbufs.len() - bytes;
            unsafe{ ptr::copy(((&mbufs[0]) as (*const u64)).offset(bytes as isize), self.vec.my_vec, remaining) };
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

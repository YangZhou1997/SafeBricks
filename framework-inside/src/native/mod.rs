pub mod mbuf;
use self::mbuf::MBuf;

pub fn mbuf_alloc_bulk(array: *mut *mut MBuf, len: u16, cnt: i32) -> i32
{
    unsafe{        
        for i in 0..cnt {
            // *(array.offset(i as isize)) = &mut MBuf::new(len as u32);
        }
    }
    0
}
pub fn mbuf_free_bulk(array: *mut *mut MBuf, cnt: i32) -> i32
{
    unsafe{        
        for i in 0..cnt {
            // drop(*(array.offset(i as isize)));
        }
    }
    0
}
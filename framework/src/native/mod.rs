pub(crate) mod mbuf;


pub fn mbuf_alloc_bulk(array: *mut *mut MBuf, len: u16, cnt: i32) -> i32
{
    // assert!(cnt as u32 == array.len());
    unsafe{        
        for i in [0..cnt] { // pkt_ref: *mut Mbuf
            (*((*array)[i])).new(len);
        }
    }
    cnt
}
pub fn mbuf_free_bulk(array: *mut *mut MBuf, cnt: i32) -> i32
{

}
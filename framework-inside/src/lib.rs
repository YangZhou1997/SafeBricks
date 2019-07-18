#![allow(safe_packed_borrows)]
#![feature(alloc)]
// Used for cache alignment.
#![feature(allocator_api)]
#![feature(asm)]
#![feature(box_syntax)]
#![feature(const_fn)]
// FIXME: Figure out if this is really the right thing here.
#![feature(ptr_internals)]
#![feature(specialization)]
#![feature(type_ascription)]
#![recursion_limit = "1024"]

// For cache aware allocation
extern crate alloc;
extern crate config as config_rs;
extern crate crossbeam;
#[cfg_attr(test, macro_use)]
extern crate failure;
extern crate fallible_iterator;
extern crate fnv;
extern crate hex;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
#[cfg(unix)]
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate twox_hash;

#[cfg(test)]
#[macro_use]
extern crate proptest; 

// extern crate openssl;

// need these first so other modules in netbricks can use the macros
#[macro_use]
pub mod common;
pub mod allocators;
pub mod config;
pub mod interface;
pub mod scheduler;
#[allow(dead_code)]
mod native;
mod native_include;
pub mod operators;
pub mod packets;
// pub mod shared_state;
// pub mod state;
// pub mod shared_ring;
pub mod utils;
pub mod runtime;
pub mod heap_ring;

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
#[macro_use]
extern crate clap;
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
extern crate net2;
#[cfg(unix)]
extern crate nix;
extern crate rayon;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio;
extern crate tokio_signal;
extern crate twox_hash;

extern crate openssl;

// need these first so other modules in netbricks can use the macros
#[macro_use]
pub mod common;
#[allow(dead_code)]
mod native;
mod native_include;
pub mod packets;

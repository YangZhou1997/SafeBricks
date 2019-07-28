#![feature(rustc_private)]

extern crate libc;
#[macro_use]
extern crate lazy_static;
extern crate ctrlc;
extern crate aesm_client;
extern crate byteorder;
extern crate enclave_runner;
extern crate sgxs_loaders;
extern crate core_affinity;
#[macro_use]
extern crate clap;
extern crate config as config_rs;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate sharedring;
extern crate cc;
extern crate tokio;

pub mod config;
pub mod haproxy;
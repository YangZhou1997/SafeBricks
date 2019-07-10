extern crate colored;
extern crate fnv;
#[macro_use]
extern crate lazy_static;
extern crate netbricks;
extern crate rand;
use self::lpm::*;
use netbricks::common::Result;
use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::runtime::Runtime;
use netbricks::scheduler::Scheduler;
use std::fmt::Display;
// use colored::*;
// use std::net::Ipv4Addr;
mod lpm;

fn install<T, S>(ports: Vec<T>, sched: &mut S)
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
    S: Scheduler + Sized,
{
    println!("Receiving started");
    for port in &ports {
        println!("Receiving port {}", port);
    }

    let pipelines: Vec<_> = ports
        .iter()
        .map(|port| {
            ReceiveBatch::new(port.clone())
                .map(lpm)
                // .group_by(
                //     |v4| LOOKUP_TABLE.read().unwrap().lookup_entry(v4.src()) as usize,
                //     |groups| {
                //         compose!(groups,
                //                  0 => |group| {
                //                      group.for_each(|_p| {
                //                         let info_fmt = format!("{}", p.src()).magenta().bold();
                //                         println!("{}", info_fmt);
                //                          Ok(())
                //                      })
                //                  },
                //                  1 => |group| {
                //                      group.for_each(|_p| {
                //                         let info_fmt = format!("{}", p.src()).red().bold();
                //                         println!("{}", info_fmt);
                //                          Ok(())
                //                      })
                //                  },
                //                  2 => |group| {
                //                      group.for_each(|_p| {
                //                         let info_fmt = format!("{}", p.src()).blue().bold();
                //                         println!("{}", info_fmt);
                //                          Ok(())
                //                      })
                //                  }
                //         );
                //     },
                // )
                .send(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn main() -> Result<()> {
    let configuration = load_config()?;
    println!("{}", configuration);
    let mut runtime = Runtime::init(&configuration)?;
    runtime.add_pipeline_to_run(install);
    runtime.execute()
}

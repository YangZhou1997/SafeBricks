use allocators::CacheAligned;
use common::*;
use config::NetBricksConfiguration;
use failure::Fail;
// use interface::dpdk::{init_system, init_thread};
// use interface::{PmdPort, PortQueue, VirtualPort, VirtualQueue};
use interface::{SimulatePort, SimulateQueue};
use scheduler::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle, Thread};

// type AlignedPortQueue = CacheAligned<PortQueue>;
// type AlignedVirtualQueue = CacheAligned<VirtualQueue>;
type AlignedSimulateQueue = CacheAligned<SimulateQueue>;

/// A handle to schedulers paused on a barrier.
pub struct BarrierHandle<'a> {
    threads: Vec<&'a Thread>,
}

impl<'a> BarrierHandle<'a> {
    /// Release all threads. This consumes the handle as expected.
    pub fn release(self) {
        for thread in &self.threads {
            thread.unpark();
        }
    }

    /// Allocate a new BarrierHandle with threads.
    pub fn with_threads(threads: Vec<&'a Thread>) -> BarrierHandle {
        BarrierHandle { threads }
    }
}

#[derive(Debug, Fail)]
#[fail(display = "Port configuration error: {}", _0)]
pub struct PortError(String);

/// `NetBricksContext` contains handles to all schedulers, and provides mechanisms for coordination.
#[derive(Default)]
pub struct NetBricksContext {
    pub ports: Vec<Arc<SimulatePort>>,
    pub rx_queues: Vec<CacheAligned<SimulateQueue>>,
    pub active_cores: Vec<i32>,
}

impl NetBricksContext {

    /// Run a function (which installs a pipeline) on the first core in the system, blocking. 
    pub fn run<T>(&mut self, run: Arc<T>, npkts: u64)
    where
        T: Fn(Vec<AlignedSimulateQueue>, &mut StandaloneScheduler) + Send + Sync + 'static,
    {
        let mut sched = StandaloneScheduler::new(npkts);
        let boxed_run = run.clone();
        let ports = self.rx_queues.clone();
        sched.run(Arc::new(move |s| {
            boxed_run(ports.clone(), s)
        }));
        sched.execute_loop();
    }
}

/// Initialize NetBricks, incl. handling of dpdk configuration, logging, general
/// setup.
///
/// Return a Context to Execute.
pub fn initialize_system(configuration: &NetBricksConfiguration) -> Result<NetBricksContext> {
    // init_system(configuration);
    let mut ctx: NetBricksContext = Default::default();
    let mut cores: HashSet<_> = configuration.cores.iter().cloned().collect();
    for port in &configuration.ports {
        match SimulatePort::new(port) {
            Ok(p) => {
                ctx.ports.push(p);
            }
            Err(e) => {
                return Err(PortError(format!(
                    "Port {} could not be initialized {:?}",
                    port.name, e
                ))
                .into());
            }
        }

        let port_instance = &ctx.ports[0];

        for (rx_q, core) in port.rx_queues.iter().enumerate() {
            let rx_q = rx_q as i32;
            match port_instance.new_simulate_queue(rx_q) {
                Ok(q) => {
                    ctx.rx_queues.push(q);
                }
                Err(e) => {
                    return Err(PortError(format!(
                        "Queue {} on port {} could not be \
                            initialized {:?}",
                        rx_q, port.name, e
                    ))
                    .into());
                }
            }
        }
    }
    // if configuration.strict {
    //     let other_cores: HashSet<_> = ctx.rx_queues.keys().cloned().collect();
    //     let core_diff: Vec<_> = other_cores
    //         .difference(&cores)
    //         .map(|c| c.to_string())
    //         .collect();
    //     if !core_diff.is_empty() {
    //         let missing_str = core_diff.join(", ");
    //         return Err(PortError(format!(
    //             "Strict configuration selected but core(s) {} appear \
    //              in port configuration but not in cores",
    //             missing_str
    //         ))
    //         .into());
    //     }
    // } else {
    // cores.extend(ctx.rx_queues.keys());
    // };
    // println!("initialize_system3");
    ctx.active_cores.push(0);
    Ok(ctx)
}

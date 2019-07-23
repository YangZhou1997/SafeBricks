use super::{Executable, Scheduler};
use common::*;
use std::default::Default;
use std::sync::mpsc::{sync_channel, Receiver, RecvError, SyncSender};
use std::sync::Arc;
use std::thread;
use utils;

/// Used to keep stats about each pipeline and eventually grant tokens, etc.
struct Runnable {
    pub task: Box<Executable>,
    pub cycles: u64,
    pub last_run: u64,
}

impl Runnable {
    pub fn from_task<T: Executable + 'static>(task: T) -> Runnable {
        Runnable {
            task: box task,
            cycles: 0,
            last_run: utils::rdtsc_unsafe(),
        }
    }
    pub fn from_boxed_task(task: Box<Executable>) -> Runnable {
        Runnable {
            task,
            cycles: 0,
            last_run: utils::rdtsc_unsafe(),
        }
    }
}

/// A very simple round-robin scheduler. This should really be more of a DRR scheduler.
pub struct StandaloneScheduler {
    /// The set of runnable items. Note we currently don't have a blocked queue.
    run_q: Vec<Runnable>,
    /// Next task to run.
    next_task: usize,
    /// Signal scheduler should continue executing tasks.
    execute_loop: bool,
    /// Number of packet processed so far
    npkts: u64, 
    /// Number of packet that will process
    tol_pkts: u64,
}

/// Messages that can be sent on the scheduler channel to add or remove tasks.
pub enum SchedulerCommand {
    Add(Box<Executable + Send>),
    Run(Arc<Fn(&mut StandaloneScheduler) + Send + Sync>),
    Execute,
    Shutdown,
    Handshake(SyncSender<bool>),
}

const DEFAULT_Q_SIZE: usize = 256;

impl Default for StandaloneScheduler {
    fn default() -> StandaloneScheduler {
        StandaloneScheduler::new(1024 * 1024)
    }
}

impl Scheduler for StandaloneScheduler {
    /// Add a task to the current scheduler.
    fn add_task<T: Executable + 'static>(&mut self, task: T) -> Result<usize> {
        self.run_q.push(Runnable::from_task(task));
        Ok(self.run_q.len())
    }
}

impl StandaloneScheduler {
    pub fn new(tol_pkts: u64) -> StandaloneScheduler {
        StandaloneScheduler {
            run_q: Vec::with_capacity(DEFAULT_Q_SIZE),
            next_task: 0,
            execute_loop: false,
            npkts: 0,
            tol_pkts, 
        }
    }

    pub fn run(&mut self, f: Arc<Fn(&mut StandaloneScheduler) + Send + Sync>) {
        f(self);
    }

    /// Run the scheduling loop.
    pub fn execute_loop(&mut self) {
        self.execute_loop = true;
        let mut begin_time = utils::rdtsc_unsafe();
        if !self.run_q.is_empty() {
            while self.execute_loop {
                begin_time = self.execute_internal(begin_time)
            }
        }
    }

    #[inline]
    fn execute_internal(&mut self, begin: u64) -> u64 {
        let time = {
            let task = &mut (&mut self.run_q[self.next_task]);
            self.npkts += task.task.execute() as u64;
            let end = utils::rdtsc_unsafe();
            task.cycles += end - begin;
            task.last_run = end;
            end
        };
        let len = self.run_q.len();
        let next = self.next_task + 1;
        if next == len {
            self.next_task = 0;
            // if self.npkts >= self.tol_pkts {
            //     self.execute_loop = false;
            // }
        } else {
            self.next_task = next;
        };
        time
    }

}

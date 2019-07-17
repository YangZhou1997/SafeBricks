use allocators::CacheAligned;
use common::Result;
use config::{NetBricksConfiguration};
use interface::SimulateQueue;
use scheduler::{initialize_system, NetBricksContext, StandaloneScheduler};
use std::io::{Error, ErrorKind};
use std::sync::mpsc::{sync_channel, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Runtime {
    context: NetBricksContext,
}

impl Runtime {
    /// Intializes the NetBricks context and starts the background schedulers
    pub fn init(configuration: &NetBricksConfiguration) -> Result<Runtime> {
        info!("initializing context:\n{}", configuration);
        let mut context = initialize_system(configuration)?;
        context.start_schedulers();
        Ok(Runtime {
            context,
        })
    }

    /// Runs a packet processing pipeline installer
    pub fn add_pipeline_to_run<T>(&mut self, installer: T)
    where
        T: Fn(Vec<CacheAligned<SimulateQueue>>, &mut StandaloneScheduler) + Send + Sync + 'static,
    {
        self.context.add_pipeline_to_run(Arc::new(installer));
    }

    fn shutdown(&mut self) {
        info!("shutting down context");
        self.context.shutdown();
    }
    
    /// Executes tasks and pipelines
    ///
    /// If a timeout is provided through command line argument `--duration`,
    /// the runtime will wait for specified value in seconds and then terminate
    /// the process. Otherwise, it will wait for a Unix signal before exiting.
    /// By default, any Unix signal received will end the process. To change
    /// this behavior, use `set_on_signal` to customize signal handling.
    pub fn execute(&mut self) -> Result<()> {
        self.context.execute();
        Ok(())
    }

    pub fn wait(&mut self) -> Result<()> {
        self.context.wait();
        Ok(())
    }
}

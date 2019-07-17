use config_rs::{Config, ConfigError, File, FileFormat, Source, Value};
use std::collections::HashMap;
use std::fmt;

pub const DEFAULT_POOL_SIZE: u32 = 2048 - 1;
pub const DEFAULT_CACHE_SIZE: u32 = 32;
pub const NUM_RXD: i32 = 128;
pub const NUM_TXD: i32 = 128;
// pub const NUM_RXD: i32 = 512;
// pub const NUM_TXD: i32 = 512;

/// NetBricks configuration
#[derive(Debug, Default, Deserialize, PartialEq)]
pub struct NetBricksConfiguration {
    /// Name, this is passed on to DPDK. If you want to run multiple DPDK apps,
    /// this needs to be unique per application.
    pub name: String,
    /// Should this process be run as a secondary process or a primary process?
    pub secondary: bool,
    /// Where should the main thread (for the examples this just sits around and
    /// prints packet counts) be run.
    pub primary_core: i32,
    /// Cores that can be used by NetBricks. Note that currently we will add any
    /// cores specified in the ports configuration to this list, unless told not
    /// to using the next option.
    pub cores: Vec<i32>,
    /// Use the core list as a strict list, i.e., error out if any cores with an
    /// rxq or txq are not specified on the core list. This is set to false by
    /// default because of laziness.
    pub strict: bool,
    /// A set of ports to be initialized.
    pub ports: Vec<PortConfiguration>,
    /// Memory pool size: sizing this pool is a bit complex; too big and you might
    /// affect caching behavior, too small and you limit how many packets are in
    /// your system overall.
    pub pool_size: u32,
    /// Size of the per-core mempool cache.
    pub cache_size: u32,
    /// Custom DPDK arguments.
    pub dpdk_args: Option<String>,
}

impl fmt::Display for NetBricksConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ports = self
            .ports
            .iter()
            .map(|p| format!("\t{}", p))
            .collect::<Vec<_>>()
            .join("\n");

        write!(
            f,
            "name: {}, secondary: {}, pool size: {}, cache size: {}\nprimary core: {}, cores: {:?}, strict: {}\nports:\n{}\nDPDK args: {:?}",
            self.name,
            self.secondary,
            self.pool_size,
            self.cache_size,
            self.primary_core,
            self.cores,
            self.strict,
            ports,
            self.dpdk_args,
        )
    }
}

/// Port (network device) configuration
#[derive(Debug, Default, Deserialize, PartialEq)]
pub struct PortConfiguration {
    /// Name. The exact semantics vary by backend. For DPDK, we allow things of the form:
    ///    <PCI ID> : Hardware device with PCI ID
    ///    dpdk:<PMD Descriptor>: PMD driver with arguments
    ///    bess:<port_name>: BESS RingVport with name.
    ///    ovs:<port_id>: OVS ring with ID.
    pub name: String,
    /// Core on which receive node for a given queue lives.
    pub rx_queues: Vec<i32>,
    /// Core on which sending node lives.
    pub tx_queues: Vec<i32>,
    /// Number of RX descriptors to use.
    pub rxd: i32,
    /// Number of TX descriptors to use.
    pub txd: i32,
    pub loopback: bool,
    pub tso: bool,
    pub csum: bool,
}

impl fmt::Display for PortConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "name: {}, rxq: {:?}, txq: {:?}, rxd: {}, txd: {}, loopback: {}, tso: {}, csum: {}",
            self.name,
            self.rx_queues,
            self.tx_queues,
            self.rxd,
            self.txd,
            self.loopback,
            self.tso,
            self.csum,
        )
    }
}



static DEFAULT_TOML: &'static str = r#"
    name = "netbricks-inside"
    secondary = false
    primary_core = 0
    cores = [0]
    strict = false
    pool_size = 512
    cache_size = 32
    [[ports]]
        name = "SimulateQueue"
        rx_queues = [0]
        tx_queues = [0]
        rxd = 128
        txd = 128
        loopback = false
        tso = false
        csum = false
    duration = 0
"#;

/// Loads the configuration
///
/// Configuration can be specified through either a file or command
/// line. Command line arguments will have precedence over settings
/// from the configuration file.
pub fn load_config() -> Result<NetBricksConfiguration, ConfigError> {
    let mut config = Config::new();
    config.merge(File::from_str(DEFAULT_TOML, FileFormat::Toml))?;
    config.try_into()
}

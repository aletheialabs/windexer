// crates/windexer-geyser/src/lib.rs

//! wIndexer Geyser Plugin
//!
//! A high-performance Geyser plugin for Solana validators that streams data to the wIndexer network.
//! This plugin is designed to be compatible with the planned Tide architecture while maintaining
//! compatibility with the current Geyser plugin interface.
//!
//! It uses memory-mapped access where possible and leverages the libp2p gossipsub network for
//! efficient data distribution.

use {
    agave_geyser_plugin_interface::{
        geyser_plugin_interface::GeyserPlugin,
    },
    log::{Log, LevelFilter},
    solana_sdk::{
        pubkey::Pubkey,
        signature::Signature,
        clock::Slot,
    },
    std::{
        fmt::{Debug, Formatter, Result as FmtResult},
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    },
    anyhow::Result,
    crossbeam_channel::{Receiver, Sender, unbounded},
    log::{debug, error, info, warn},
    plugin::WindexerGeyserPlugin,
};

mod config;
mod plugin;
mod processor;
mod publisher;
mod metrics;
#[cfg(test)]
mod tests;

// Public exports
pub use config::GeyserPluginConfig;
pub use metrics::Metrics;
pub use processor::{AccountHandler, TransactionHandler, BlockHandler};

#[no_mangle]
#[allow(improper_ctypes_definitions)]
/// Create and return a wIndexer Geyser plugin.
///
/// # Safety
///
/// This function returns a raw pointer to the plugin instance that will be used by the validator.
/// The validator is responsible for managing the lifetime of this pointer.
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin = WindexerGeyserPlugin::new();
    let boxed = Box::new(plugin);
    Box::into_raw(boxed)
}

#[derive(Debug)]
pub struct ShutdownFlag(AtomicBool);

impl ShutdownFlag {
    pub fn new() -> Self {
        Self(AtomicBool::new(false))
    }

    pub fn shutdown(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_shutdown(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub struct PluginVersion {
    pub version: &'static str,
    pub build_timestamp: u64,
    pub rust_version: &'static str,
}

impl PluginVersion {
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            build_timestamp: env!("BUILD_TIMESTAMP").parse().unwrap_or(0),
            rust_version: env!("RUST_VERSION"),
        }
    }
}

impl Debug for PluginVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("PluginVersion")
            .field("version", &self.version)
            .field("build_timestamp", &self.build_timestamp)
            .field("rust_version", &self.rust_version)
            .finish()
    }
}
#!/bin/bash
set -e

echo "🔧 Setting up minimal Geyser plugin..."

mkdir -p config/geyser
mkdir -p windexer_geyser_setup

if [ ! -f "config/geyser/plugin-keypair.json" ]; then
    solana-keygen new --no-passphrase -o config/geyser/plugin-keypair.json > /dev/null 2>&1
fi

cat > config/geyser/minimal-config.json << EOF
{
  "libpath": "../../target/debug/libwindexer_geyser.so",
  "keypair": "./plugin-keypair.json",
  "no_processors": true,
  "accounts_selector": { "accounts": [] },
  "transaction_selector": { "mentions": [] },
  "thread_count": 1,
  "batch_size": 1
}
EOF

cat > crates/windexer-geyser/src/mock_plugin.rs << EOF
//! Mock Plugin Implementation

use {
    crate::{
        config::GeyserPluginConfig,
        plugin_driver::GeyserPluginDriver,
    },
    log::{info, warn},
    solana_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPlugin, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
        ReplicaTransactionInfoVersions, SlotStatus,
    },
    std::{fmt::Debug, str::FromStr, sync::Arc},
};

#[derive(Debug)]
pub struct MockWindexerPlugin;

impl GeyserPlugin for MockWindexerPlugin {
    fn name(&self) -> &'static str {
        "MockWindexerPlugin"
    }

    fn on_load(&mut self, config_file: &str) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        info!("Mock plugin loaded with config: {}", config_file);
        Ok(())
    }

    fn on_unload(&mut self) {
        info!("Mock plugin unloaded");
    }

    // Add other required empty implementations
    // ...
}
EOF

cat > crates/windexer-geyser/src/lib.rs.new << EOF
// Original lib.rs content with mock plugin export

pub mod mock_plugin;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
/// Geyser plugin entrypoint
pub unsafe extern "C" fn _create_plugin() -> *mut dyn solana_geyser_plugin_interface::geyser_plugin_interface::GeyserPlugin {
    let plugin = mock_plugin::MockWindexerPlugin {};
    let plugin_box = Box::new(plugin);
    Box::into_raw(plugin_box)
}

// ... Rest of original lib.rs content
EOF

cargo build --package windexer-geyser

echo "✅ Setup complete!"
echo "Running validator with minimal plugin..."
RUST_BACKTRACE=1 RUST_LOG=solana_geyser_plugin_manager=debug,windexer_geyser=debug \
solana-test-validator \
  --geyser-plugin-config config/geyser/minimal-config.json \
  --reset 